//! The hot reload's host through the real `EditorGame`: what the world puts in the watched
//! set, the editor's own saves as the watcher's baseline until the worker acknowledges them,
//! a script reloaded in a session with its errors reaching the console every time, and the
//! scene's reload rules — clean, broken, dirty, mid-session, another scene open, a gesture
//! live, a name that reads as a sheet's.

use std::path::PathBuf;
use std::time::Duration;

use ecs::script::{ScriptRef, Scripts};
use ecs::sprite_components::SpriteAnimation;
use ecs::World;
use editor::console::Severity;
use editor::external_tools::{FileChange, FileWatch, Location, OpenRequest, WatchEvent};
use editor::ConfirmChoice;
use engine_core::assets::{AssetConfig, AssetManager};
use engine_core::scripting::ScriptRunner;
use engine_core::test_support::{save_to_ron, test_texture_path};
use glam::Vec2;
use winit::keyboard::KeyCode;

use super::hot_reload::{ReloadTargets, WatchOwner};
use super::play_session::PausedEdits;
use super::scene_confirm::PendingSceneAction;
use super::test_support::{dirty_editor, editor_game, press_mouse, press_play, spawn_at, ui_frame, DummyGame};
use super::EditorGame;

/// A worker that never polls: every change a test dispatches is the test's own, and its
/// saves are noted and acknowledged by a real thread, as the editor's are.
const IDLE_POLL_INTERVAL: Duration = Duration::from_secs(3600);

/// An editor over a temporary project, with the asset manager and the runner a frame lends,
/// its watcher's worker already started as the first frame's drain starts it.
pub(super) struct Session {
    pub(super) project: tempfile::TempDir,
    pub(super) editor: EditorGame<DummyGame>,
    pub(super) world: World,
    pub(super) assets: AssetManager,
    pub(super) scripts: ScriptRunner,
}

impl Session {
    pub(super) fn new() -> Self {
        Self::with_editor(editor_game())
    }

    pub(super) fn with_editor(mut editor: EditorGame<DummyGame>) -> Self {
        let project = tempfile::tempdir().expect("a temporary project");
        let base = project.path().to_string_lossy().into_owned();
        editor.asset_base = project.path().to_path_buf();
        editor.hot_reload.worker = Some(super::hot_reload::spawn_worker(IDLE_POLL_INTERVAL));
        let assets = AssetManager::headless(AssetConfig { base_path: base.clone(), ..AssetConfig::default() });
        let mut world = World::new();
        let mut scripts = ScriptRunner::new();
        scripts.reset(&mut world, &base);
        Self { project, editor, world, assets, scripts }
    }

    pub(super) fn path(&self, relative: &str) -> PathBuf {
        self.project.path().join(relative)
    }

    pub(super) fn write(&self, relative: &str, text: &str) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().expect("a parent folder")).expect("the folder");
        std::fs::write(path, text).expect("the file");
    }

    /// Save the world as the scene at `relative`, through the editor's one choke point, and
    /// let the worker's acknowledgement of the save arrive: until it does, a change for that
    /// path is one the worker read before the save.
    pub(super) fn save_scene(&mut self, relative: &str) {
        let path = self.path(relative);
        self.editor.save_scene_with(&mut self.world, &test_texture_path, path).expect("the save");
        let acknowledgements = self.acknowledgements();
        self.dispatch_events(acknowledgements, false);
    }

    /// The worker's events once it has acknowledged every save the editor sent it, within a
    /// bounded wait; none when no worker runs.
    pub(super) fn acknowledgements(&self) -> Vec<WatchEvent> {
        let Some(worker) = &self.editor.hot_reload.worker else { return Vec::new() };
        let awaiting: u32 = self.editor.hot_reload.awaiting_note.values().sum();
        let mut events = Vec::new();
        for _ in 0..1000 {
            events.extend(worker.take_events());
            if events.iter().filter(|event| matches!(event, WatchEvent::Noted(_))).count() >= awaiting as usize {
                return events;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("the worker acknowledged {events:?} of {awaiting} saves within its bounded wait");
    }

    /// The watcher's tick: the set as the world names it now.
    pub(super) fn watch_now(&mut self) {
        self.editor.hot_reload.watched = self.editor.watched_paths(&self.world);
    }

    pub(super) fn dispatch(&mut self, changes: &[FileChange], edit_field_focused: bool) {
        self.dispatch_events(changes.iter().cloned().map(WatchEvent::Change).collect(), edit_field_focused);
    }

    /// One drain of `events`, in the order the worker sent them.
    pub(super) fn dispatch_events(&mut self, events: Vec<WatchEvent>, edit_field_focused: bool) {
        self.editor.hot_reload.incoming.extend(events);
        let mut targets = ReloadTargets {
            world: &mut self.world,
            assets: &mut self.assets,
            scripts: &mut self.scripts,
            edit_field_focused,
        };
        self.editor.dispatch_file_changes(&mut targets);
    }

    pub(super) fn changed(&mut self, relative: &str) {
        let path = self.path(relative);
        self.dispatch(&[FileChange::Changed(path)], false);
    }

    pub(super) fn console(&self) -> Vec<(Severity, String)> {
        let entries = self.editor.editor.console.log.entries();
        entries.iter().map(|entry| (entry.severity, entry.text.clone())).collect()
    }

    pub(super) fn console_mentions(&self, text: &str) -> bool {
        self.console().iter().any(|(_, line)| line.contains(text))
    }
}

fn rhai(source_path: &str) -> ScriptRef {
    let mut script = ScriptRef::new("mover");
    script.source_path = source_path.to_string();
    script
}

fn scripted(world: &mut World, script: ScriptRef) {
    let entity = world.create_entity();
    world.add_component(&entity, Scripts(vec![script])).ok();
}

/// A scene on disk holding `count` entities.
fn scene_of(count: usize) -> String {
    let mut world = World::new();
    for index in 0..count {
        spawn_at(&mut world, Vec2::new(index as f32, 0.0));
    }
    save_to_ron(&world)
}

#[test]
fn a_touched_script_is_reported_and_an_unrelated_file_is_not() {
    let mut session = Session::new();
    session.write("scripts/paddle.rhai", "fn update(me, view, params, cmd, dt) {}\n");
    session.write("notes.txt", "unwatched");
    scripted(&mut session.world, rhai("scripts/paddle.rhai"));

    let mut watch = FileWatch::new();
    watch.set_registered(session.editor.watched_paths(&session.world).into_keys().collect());
    assert!(watch.poll().is_empty());
    session.write("scripts/paddle.rhai", "fn update(me, view, params, cmd, dt) { }\n// edited\n");
    session.write("notes.txt", "edited, and still unwatched");
    watch.poll();

    assert_eq!(watch.poll(), vec![FileChange::Changed(session.path("scripts/paddle.rhai"))]);
}

#[test]
fn a_native_script_and_a_placeholder_sheet_watch_nothing() {
    let session = Session::new();
    let mut world = World::new();
    scripted(&mut world, ScriptRef::new("engine::rotate"));
    let no_sheet = world.create_entity();
    world.add_component(&no_sheet, SpriteAnimation::default()).ok();
    let empty_sheet = world.create_entity();
    world.add_component(&empty_sheet, SpriteAnimation { sheet: Some(String::new()), ..SpriteAnimation::default() }).ok();

    assert!(session.editor.watched_paths(&world).is_empty());
}

#[test]
fn two_entities_on_one_script_are_one_path_and_a_sheet_is_watched_by_its_sidecar() {
    let session = Session::new();
    let mut world = World::new();
    scripted(&mut world, rhai("scripts/paddle.rhai"));
    scripted(&mut world, rhai("scripts/paddle.rhai"));
    let brick = world.create_entity();
    let sheet = SpriteAnimation { sheet: Some("sprites/brick.png".to_string()), ..SpriteAnimation::default() };
    world.add_component(&brick, sheet).ok();

    let watched = session.editor.watched_paths(&world);
    let paddle = watched.get(&session.path("scripts/paddle.rhai")).expect("the script is watched");
    assert_eq!(paddle.iter().collect::<Vec<_>>(), vec![&WatchOwner::Script("scripts/paddle.rhai".to_string())]);
    let brick = watched.get(&session.path("sprites/brick.sheet.ron")).expect("the sidecar is watched");
    assert_eq!(brick.iter().collect::<Vec<_>>(), vec![&WatchOwner::Sheet("sprites/brick.png".to_string())]);
    assert_eq!(watched.len(), 2);
}

#[test]
fn ctrl_s_is_the_watchers_baseline_and_yields_no_change() {
    let mut session = Session::new();
    spawn_at(&mut session.world, Vec2::ZERO);
    session.save_scene("scenes/level.scene.ron");
    let scene = session.path("scenes/level.scene.ron");
    let mut watch = FileWatch::new();
    watch.set_registered(session.editor.watched_paths(&session.world).into_keys().collect());
    watch.poll();

    spawn_at(&mut session.world, Vec2::ONE);
    session.editor.save_scene_with(&mut session.world, &test_texture_path, scene.clone()).expect("Ctrl+S");
    assert!(watch.poll().is_empty(), "the poll catches the save mid-debounce");
    let stamp = common::vfs::change_stamp(&scene).ok().flatten();
    watch.note_written(&scene, stamp.expect("the file keeps a stamp"));

    assert!(watch.poll().is_empty());
    assert!(watch.poll().is_empty(), "the editor's own save never comes back as a change");
}

#[test]
fn ctrl_s_with_a_change_for_that_path_already_queued_reloads_nothing() {
    let mut session = Session::new();
    spawn_at(&mut session.world, Vec2::ZERO);
    session.save_scene("level.scene.ron");
    session.watch_now();
    let scene = session.path("level.scene.ron");
    session.editor.hot_reload.incoming.push(WatchEvent::Change(FileChange::Changed(scene)));

    spawn_at(&mut session.world, Vec2::ONE);
    session.save_scene("level.scene.ron");
    session.dispatch(&[], false);

    assert_eq!(session.world.entity_count(), 2, "the world the editor saved is the one open");
    assert!(!session.console_mentions("changed on disk"), "{:?}", session.console());
    assert_eq!(session.editor.hot_reload.waiting_scene, None);
}

#[test]
fn a_change_the_worker_sends_after_the_saves_drain_but_before_its_acknowledgement_reloads_nothing() {
    let mut world = World::new();
    let editor = dirty_editor(&mut world);
    let mut session = Session::with_editor(editor);
    session.world = world;
    session.save_scene("level.scene.ron");
    session.watch_now();
    let scene = session.path("level.scene.ron");

    session.editor.save_scene_with(&mut session.world, &test_texture_path, scene.clone()).expect("Ctrl+S");
    session.dispatch(&[], false);
    let mut events = vec![WatchEvent::Change(FileChange::Changed(scene))];
    events.extend(session.acknowledgements());
    session.dispatch_events(events, false);

    assert!(!session.console_mentions("changed on disk"), "{:?}", session.console());
    assert!(session.editor.command_history.can_undo(), "a reload of its own bytes would have taken the undo history");

    session.write("level.scene.ron", &scene_of(3));
    session.changed("level.scene.ron");
    assert_eq!(session.world.entity_count(), 3, "a change after the acknowledgement is another program's");
}

#[test]
fn two_saves_hold_a_change_until_the_worker_has_acknowledged_both() {
    let mut session = Session::new();
    spawn_at(&mut session.world, Vec2::ZERO);
    session.save_scene("level.scene.ron");
    session.watch_now();
    let scene = session.path("level.scene.ron");
    for _ in 0..2 {
        session.editor.save_scene_with(&mut session.world, &test_texture_path, scene.clone()).expect("Ctrl+S");
    }
    let mut acknowledgements = session.acknowledgements().into_iter();
    let change = WatchEvent::Change(FileChange::Changed(scene));

    session.dispatch_events(vec![acknowledgements.next().expect("the first save's"), change.clone()], false);
    assert!(!session.console_mentions("changed on disk"), "{:?}", session.console());

    session.dispatch_events(vec![acknowledgements.next().expect("the second save's"), change], false);
    assert!(session.console_mentions("changed on disk"), "{:?}", session.console());
}

#[test]
fn a_save_before_the_worker_starts_holds_back_no_later_change() {
    let mut session = Session::new();
    session.editor.hot_reload.worker = None;
    spawn_at(&mut session.world, Vec2::ZERO);
    session.save_scene("level.scene.ron");
    session.watch_now();
    session.editor.hot_reload.worker = Some(super::hot_reload::spawn_worker(IDLE_POLL_INTERVAL));

    session.write("level.scene.ron", &scene_of(3));
    session.changed("level.scene.ron");

    assert_eq!(session.world.entity_count(), 3, "the first frame's save left nothing for an acknowledgement to answer");
}

#[test]
fn a_save_dismisses_the_reload_prompt_parked_for_that_file() {
    let mut world = World::new();
    let editor = dirty_editor(&mut world);
    let mut session = Session::with_editor(editor);
    session.world = world;
    let path = session.path("level.scene.ron");
    session.editor.editor.set_scene_path(Some(path.clone()));
    session.write("level.scene.ron", &scene_of(3));
    session.watch_now();
    session.changed("level.scene.ron");
    assert_eq!(session.editor.scene_confirm.pending_action, Some(PendingSceneAction::ReloadFromDisk(path)));

    session.save_scene("level.scene.ron");

    assert_eq!(session.editor.scene_confirm.pending_action, None, "the file holds the editor's own bytes now");
    assert_eq!(session.world.entity_count(), 1, "the edited world stays, and is the one saved");
}

#[test]
fn a_save_drops_a_scene_change_waiting_for_a_focused_field() {
    let mut session = Session::new();
    spawn_at(&mut session.world, Vec2::ZERO);
    session.save_scene("level.scene.ron");
    session.watch_now();
    let scene = session.path("level.scene.ron");
    session.write("level.scene.ron", &scene_of(3));
    session.dispatch(&[FileChange::Changed(scene.clone())], true);
    assert_eq!(session.editor.hot_reload.waiting_scene, Some(scene));

    spawn_at(&mut session.world, Vec2::ONE);
    session.save_scene("level.scene.ron");
    session.dispatch(&[], false);

    assert_eq!(session.world.entity_count(), 2, "the world the editor saved is the one open");
    assert!(!session.console_mentions("Scene reloaded from disk"), "{:?}", session.console());
    assert_eq!(session.editor.hot_reload.waiting_scene, None);
}

#[test]
fn a_scene_saved_under_a_sheet_sidecars_name_reloads_as_the_scene() {
    let mut session = Session::new();
    spawn_at(&mut session.world, Vec2::ZERO);
    session.save_scene("level.sheet.ron");
    session.watch_now();

    session.write("level.sheet.ron", &scene_of(3));
    session.changed("level.sheet.ron");

    assert_eq!(session.world.entity_count(), 3);
    assert!(session.console_mentions("changed on disk (reload scene)"), "{:?}", session.console());
}

#[test]
fn a_script_changed_while_editing_is_a_console_line_only() {
    let mut session = Session::new();
    session.write("scripts/paddle.rhai", "fn update(me, view, params, cmd, dt) {\n    let x = ;\n}\n");
    scripted(&mut session.world, rhai("scripts/paddle.rhai"));
    session.watch_now();

    session.changed("scripts/paddle.rhai");

    assert_eq!(
        session.console(),
        vec![(Severity::Info, "scripts/paddle.rhai changed on disk (recompile script)".to_string())],
        "nothing compiles until the next session starts"
    );
}

#[test]
fn a_reload_in_play_reaches_the_console_every_time_a_broken_save_comes_back() {
    let mut session = Session::new();
    let good = "fn update(me, view, params, cmd, dt) {}\n";
    let broken = "fn update(me, view, params, cmd, dt) {\n    let x = ;\n}\n";
    session.write("scripts/paddle.rhai", good);
    scripted(&mut session.world, rhai("scripts/paddle.rhai"));
    session.watch_now();
    press_play(&mut session.editor, &mut session.world);
    let at_line_two = OpenRequest { path: PathBuf::from("scripts/paddle.rhai"), location: Location::Line { line: 2, column: None } };

    for _ in 0..2 {
        session.write("scripts/paddle.rhai", broken);
        session.changed("scripts/paddle.rhai");
        let entry = session.editor.editor.console.log.entries().back().cloned().expect("the error");
        assert_eq!((entry.severity, entry.location), (Severity::Error, Some(at_line_two.clone())), "{}", entry.text);
        assert!(entry.text.starts_with("scripts/paddle.rhai:2: [Syntax]"), "{}", entry.text);

        session.write("scripts/paddle.rhai", good);
        session.changed("scripts/paddle.rhai");
        assert!(session.console().last().is_some_and(|(_, line)| line == "scripts/paddle.rhai reloaded"));
    }
}

#[test]
fn a_clean_scene_changed_on_disk_reloads_and_its_history_goes_with_the_old_document() {
    let mut session = Session::new();
    spawn_at(&mut session.world, Vec2::ZERO);
    session.save_scene("level.scene.ron");
    session.watch_now();

    session.write("level.scene.ron", &scene_of(3));
    session.changed("level.scene.ron");

    assert_eq!(session.world.entity_count(), 3);
    assert!(session.console_mentions("Scene reloaded from disk"), "{:?}", session.console());
    assert!(!session.editor.command_history.can_undo());
}

#[test]
fn a_scene_file_that_does_not_load_leaves_the_live_world_and_says_so() {
    let mut session = Session::new();
    spawn_at(&mut session.world, Vec2::ZERO);
    session.save_scene("level.scene.ron");
    session.watch_now();

    session.write("level.scene.ron", "SceneData( this is not a scene");
    session.changed("level.scene.ron");

    assert_eq!(session.world.entity_count(), 1);
    let (severity, line) = session.console().last().cloned().expect("the report");
    assert_eq!(severity, Severity::Error);
    assert!(line.contains("did not load, the open scene is kept"), "{line}");
}

#[test]
fn a_dirty_scene_asks_and_enter_or_escape_keeps_the_edits_while_reload_is_a_click() {
    let mut world = World::new();
    let editor = dirty_editor(&mut world);
    let mut session = Session::with_editor(editor);
    session.world = world;
    let path = session.path("level.scene.ron");
    session.editor.editor.set_scene_path(Some(path.clone()));
    session.write("level.scene.ron", &scene_of(3));
    session.watch_now();

    for key in [KeyCode::Enter, KeyCode::Escape] {
        session.changed("level.scene.ron");
        assert_eq!(session.editor.scene_confirm.pending_action, Some(PendingSceneAction::ReloadFromDisk(path.clone())));

        assert!(session.editor.confirm_dialog_consumes_key(key), "the prompt takes {key:?}");
        let choice = session.editor.scene_confirm.pending_choice.take();
        assert_eq!(choice, Some(ConfirmChoice::Cancel), "{key:?} keeps mine: the prompt appears unasked");
        let choice = choice.expect("the key's choice");
        session.editor.resolve_scene_reload_choice(&mut session.world, &mut session.assets, &path, choice);
        assert_eq!(session.editor.scene_confirm.pending_action, None);
        assert_eq!(session.world.entity_count(), 1, "the edited world stays");
        assert!(session.editor.command_history.is_dirty(), "with its edits");
        assert!(session.console_mentions("changed on disk (reload scene)"), "the line stays in the console");
        assert!(session.editor.watched_paths(&session.world).contains_key(&path), "and the file stays watched");
    }

    session.changed("level.scene.ron");
    session.editor.resolve_scene_reload_choice(&mut session.world, &mut session.assets, &path, ConfirmChoice::Confirm);
    assert_eq!(session.world.entity_count(), 3, "Reload takes the file");
}

#[test]
fn a_scene_changed_mid_session_waits_for_stop_and_then_reloads() {
    let mut session = Session::new();
    spawn_at(&mut session.world, Vec2::ZERO);
    session.save_scene("level.scene.ron");
    session.watch_now();
    press_play(&mut session.editor, &mut session.world);

    session.write("level.scene.ron", &scene_of(3));
    session.changed("level.scene.ron");
    assert_eq!(session.world.entity_count(), 1, "nothing reloads under a running session");
    assert!(session.console_mentions("reloads from disk when Play stops"), "{:?}", session.console());

    session.editor.stop_play_session(&mut session.world, PausedEdits::Discard);
    session.dispatch(&[], false);
    assert_eq!(session.world.entity_count(), 3, "the restored world is replaced once Stop has run");
}

#[test]
fn a_change_to_a_scene_no_longer_open_does_nothing() {
    let mut session = Session::new();
    spawn_at(&mut session.world, Vec2::ZERO);
    session.save_scene("a.scene.ron");
    session.watch_now();
    session.write("b.scene.ron", &scene_of(2));
    let scene_b = session.path("b.scene.ron");
    session.editor.load_scene(&mut session.world, &mut session.assets, &scene_b).expect("scene B opens");

    session.write("a.scene.ron", &scene_of(5));
    session.changed("a.scene.ron");

    assert_eq!(session.world.entity_count(), 2, "scene B stays open");
    assert_eq!(session.editor.hot_reload.waiting_scene, None);
}

#[test]
fn a_live_drag_or_a_focused_field_holds_the_reload_until_it_ends() {
    let mut session = Session::new();
    let entity = spawn_at(&mut session.world, Vec2::ZERO);
    session.save_scene("level.scene.ron");
    session.watch_now();
    session.write("level.scene.ron", &scene_of(3));

    let center = Vec2::new(400.0, 300.0);
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();
    press_mouse(&mut input, center);
    ui_frame(&mut ui, &input, Vec2::new(800.0, 600.0), |ui| {
        session.editor.editor.gizmo.render(ui, center, true);
    });
    assert!(session.editor.editor.gizmo_has_priority(), "fixture: the drag is live");
    session.changed("level.scene.ron");
    assert_eq!(session.world.entity_count(), 1, "the drag's world is not pulled away");
    assert!(session.world.get::<common::Transform2D>(entity).is_some());

    session.editor.editor.gizmo.cancel();
    session.dispatch(&[], true);
    assert_eq!(session.world.entity_count(), 1, "a focused field holds it too");

    session.dispatch(&[], false);
    assert_eq!(session.world.entity_count(), 3, "it lands on the first frame with neither");
}

#[test]
fn a_deleted_file_is_a_line_and_reloads_nothing() {
    let mut session = Session::new();
    spawn_at(&mut session.world, Vec2::ZERO);
    session.save_scene("level.scene.ron");
    session.watch_now();
    let scene = session.path("level.scene.ron");
    std::fs::remove_file(&scene).expect("the delete");

    session.dispatch(&[FileChange::Deleted(scene)], false);

    assert_eq!(session.world.entity_count(), 1);
    assert!(session.console_mentions("was deleted on disk"), "{:?}", session.console());
    assert_eq!(session.editor.hot_reload.waiting_scene, None);
}

#[test]
fn the_worker_reports_a_registered_file_written_by_another_program_and_joins_on_drop() {
    let project = tempfile::tempdir().expect("a temporary project");
    let watched = project.path().join("paddle.rhai");
    let unwatched = project.path().join("other.rhai");
    std::fs::write(&watched, "").expect("the watched file");
    let worker = super::hot_reload::spawn_worker(std::time::Duration::from_millis(5));
    worker.register([watched.clone()].into());

    let mut changes = Vec::new();
    for attempt in 1..=200 {
        // Written again each attempt until the worker's first poll has read a baseline.
        std::fs::write(&watched, "x".repeat(attempt)).expect("the outside write");
        std::fs::write(&unwatched, "y".repeat(attempt)).expect("the unwatched write");
        std::thread::sleep(std::time::Duration::from_millis(25));
        changes.extend(worker.take_events());
        if !changes.is_empty() {
            break;
        }
    }

    assert!(!changes.is_empty(), "the worker reported nothing within its bounded wait");
    assert!(changes.iter().all(|change| *change == WatchEvent::Change(FileChange::Changed(watched.clone()))), "{changes:?}");
    drop(worker);
}

#[test]
fn the_worker_acknowledges_a_noted_write_whether_or_not_it_had_a_stamp() {
    let project = tempfile::tempdir().expect("a temporary project");
    let written = project.path().join("level.scene.ron");
    std::fs::write(&written, "").expect("the written file");
    let worker = super::hot_reload::spawn_worker(std::time::Duration::from_millis(5));
    worker.register([written.clone()].into());
    worker.note_written(written.clone(), common::vfs::change_stamp(&written).ok().flatten());
    worker.note_written(written.clone(), None);

    let mut events = Vec::new();
    for _ in 0..200 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        events.extend(worker.take_events());
        if events.len() >= 2 {
            break;
        }
    }

    assert_eq!(events, vec![WatchEvent::Noted(written.clone()), WatchEvent::Noted(written)]);
}
