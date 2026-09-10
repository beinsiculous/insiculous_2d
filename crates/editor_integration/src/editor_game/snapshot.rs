//! The live scene as RON, without saving it.
//!
//! The preview window needs the world the visitor is looking at, unsaved
//! edits included, while the editor keeps owning the file. The request is a
//! mailbox because the two sides run on different clocks: a JavaScript
//! caller files a generation and polls, and the editor answers on its next
//! frame.

use std::sync::{Arc, Mutex};

use ecs::World;
use engine_core::Game;

use super::scene_io::SceneIoError;
use super::EditorGame;

/// The live world serialized for the preview, and the archive entry whose
/// bytes it replaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneSnapshot {
    pub scene_entry: String,
    pub ron: String,
}

/// How one generation of a snapshot request ends. Every subscriber holds
/// this and polls it, so a request that was answered, cancelled or rejected
/// settles exactly once and reads the same for everyone waiting on it — a
/// launch and an Export joined behind it each get the answer.
#[derive(Default)]
pub struct Completion {
    outcome: Mutex<Option<Result<SceneSnapshot, String>>>,
}

impl Completion {
    fn settle(&self, outcome: Result<SceneSnapshot, String>) {
        if let Ok(mut slot) = self.outcome.lock() {
            if slot.is_none() {
                *slot = Some(outcome);
            }
        }
    }

    /// Whether this generation has ended, however it ended.
    pub fn is_settled(&self) -> bool {
        self.outcome.lock().map(|slot| slot.is_some()).unwrap_or(false)
    }

    /// The outcome, for every reader: `None` until the generation settles,
    /// then the same answer however many subscribers ask, so no poller can
    /// starve another of a result the editor produced once.
    pub fn outcome(&self) -> Option<Result<SceneSnapshot, String>> {
        self.outcome.lock().ok().and_then(|slot| slot.clone())
    }
}

/// The mailbox between the bridge and the editor frame.
///
/// One generation is in flight at a time. The shared [`Completion`] is what
/// makes a launch and an Export started behind it read the same answer: both
/// hold it before either polls, and a launch's cleanup cannot erase a result
/// the Export has not read yet.
#[derive(Default)]
pub struct SceneSnapshotRequest {
    /// The generation filed, `None` when idle.
    pending: Mutex<Option<u64>>,
    /// The generation currently subscribable, and its completion.
    completion: Mutex<Option<(u64, Arc<Completion>)>>,
}

impl SceneSnapshotRequest {
    /// File a generation. Refused while another one is pending — the editor
    /// answers one request per frame and a second would race it.
    pub fn file(&self, generation: u64) -> bool {
        // Both locks in this order everywhere they are held together.
        let Ok(mut pending) = self.pending.lock() else {
            return false;
        };
        let Ok(mut completion) = self.completion.lock() else {
            return false;
        };
        let in_flight = completion
            .as_ref()
            .map(|(_, shared)| !shared.is_settled())
            .unwrap_or(false);
        if in_flight {
            return false;
        }
        // Filing frees the previous generation's slot, so a result nobody
        // read is discarded only once someone asks for a newer one.
        *pending = Some(generation);
        *completion = Some((generation, Arc::new(Completion::default())));
        true
    }

    /// The completion every waiter on `generation` shares, or `None` when
    /// that generation is not the one in flight.
    pub fn subscribe(&self, generation: u64) -> Option<Arc<Completion>> {
        let completion = self.completion.lock().ok()?;
        match completion.as_ref() {
            Some((filed, shared)) if *filed == generation => Some(Arc::clone(shared)),
            _ => None,
        }
    }

    /// Abandon `generation`: it stops being pending and its completion
    /// settles as cancelled, so a timed-out request can never resolve with
    /// a later generation's answer.
    pub fn cancel(&self, generation: u64) {
        if let Ok(mut pending) = self.pending.lock() {
            if *pending == Some(generation) {
                *pending = None;
            }
        }
        if let Ok(completion) = self.completion.lock() {
            if let Some((filed, shared)) = completion.as_ref() {
                if *filed == generation {
                    shared.settle(Err("the snapshot was cancelled".to_string()));
                }
            }
        }
    }

    /// The generation still waiting for an answer, if any. An Export started
    /// behind a preview launch uses it to join that launch's request instead
    /// of filing a second one.
    pub fn pending_generation(&self) -> Option<u64> {
        let completion = self.completion.lock().ok()?;
        match completion.as_ref() {
            Some((generation, shared)) if !shared.is_settled() => Some(*generation),
            _ => None,
        }
    }

    /// The generation waiting for an answer, taken by the editor frame.
    pub(crate) fn take_request(&self) -> Option<u64> {
        self.pending.lock().ok().and_then(|mut pending| pending.take())
    }

    /// Deliver the editor's answer. An answer for a generation nobody is
    /// subscribed to is dropped: it was cancelled while the frame ran.
    pub(crate) fn answer(&self, generation: u64, outcome: Result<SceneSnapshot, String>) {
        if let Ok(completion) = self.completion.lock() {
            if let Some((filed, shared)) = completion.as_ref() {
                if *filed == generation {
                    shared.settle(outcome);
                }
            }
        }
    }
}

impl<G: Game> EditorGame<G> {
    /// The live world as scene RON, leaving the editor session untouched.
    ///
    /// Serializes a scratch copy rather than the live world: the naming rule
    /// that save applies through the history must run before serialization,
    /// because `scripts_to_data` drops a parameter whose target has no
    /// `Name`. Applying it to the live world here would be an unrecorded
    /// mutation, so it is applied to the scratch instead — the file, the
    /// history, the dirty mark and `scene_path` never move, including when
    /// serialization fails.
    pub(super) fn scene_snapshot(
        &mut self,
        world: &World,
        texture_path_fn: &dyn Fn(u32) -> String,
    ) -> Result<SceneSnapshot, SceneIoError> {
        if self.editor.in_play_session() {
            return Err(SceneIoError::MidSimulation);
        }

        let path = self
            .editor
            .scene_path()
            .map(|scene_path| scene_path.to_path_buf())
            .unwrap_or_else(|| self.default_scene_path());
        let scene_entry = self.scene_entry_for(&path)?;

        let mut scratch = World::new();
        editor::world_snapshot::WorldSnapshot::capture(world).restore(&mut scratch);
        engine_core::script_data::ensure_script_target_names(&mut scratch);

        let scene_name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Untitled")
            .to_string();
        let scene_data = engine_core::scene_serializer::world_to_scene_data(
            &scratch,
            &scene_name,
            self.physics_settings.clone(),
            texture_path_fn,
        );
        let ron = engine_core::scene_serializer::serialize_to_ron(&scene_data)
            .map_err(SceneIoError::Write)?;

        Ok(SceneSnapshot { scene_entry, ron })
    }

    /// The archive entry the scene at `path` occupies: its location under
    /// the asset base, prefixed `assets/` the way the exporter keys it.
    fn scene_entry_for(&self, path: &std::path::Path) -> Result<String, SceneIoError> {
        let relative = if self.asset_base.as_os_str().is_empty() {
            path
        } else {
            path.strip_prefix(&self.asset_base)
                .map_err(|_| SceneIoError::OutsideProject(path.to_path_buf()))?
        };
        let entry = relative.to_string_lossy().replace('\\', "/");
        let entry = entry.trim_start_matches('/');
        if entry.is_empty() {
            return Err(SceneIoError::OutsideProject(path.to_path_buf()));
        }
        Ok(format!("assets/{entry}"))
    }

    /// Answer a filed snapshot request, if one is waiting.
    ///
    /// Runs before the command API's drain, which skips mid-drag: this path
    /// only reads the world, so a long drag must not starve it past the
    /// bridge's five-second cap. A mid-drag answer therefore captures the
    /// drag where it stands.
    pub(super) fn answer_scene_snapshot(&mut self, ctx: &mut engine_core::contexts::GameContext) {
        let Some(request) = self.scene_snapshot.clone() else {
            return;
        };
        let Some(generation) = request.take_request() else {
            return;
        };
        let texture_path_fn = |handle: u32| -> String {
            super::scene_io::texture_ref_for_save(handle, ctx.assets.texture_path(handle))
        };
        let outcome = self
            .scene_snapshot(ctx.world, &texture_path_fn)
            .map_err(|error| error.to_string());
        request.answer(generation, outcome);
    }
}
