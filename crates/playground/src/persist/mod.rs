//! Write persistence state machine for the playground.
//!
//! Provides the store's one queue of compare-and-swap puts and removals (`queue.rs`),
//! write-epoch gating, and dirty/banner synchronisation.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::projects::ProjectManifest;
use crate::store::{ProjectStore, StoredFile};

mod queue;

use queue::{HeadOperation, HeadStatus, QueuedOperation};

/// States a file path can occupy in the persistence state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathState {
    /// Stored bytes match local edits.
    Idle,
    /// This path's operation is the one the store is running.
    InFlight,
    /// More operations for this path wait in the queue: behind its own in-flight one,
    /// or behind another path's.
    Queued,
    /// This path's operation failed with Backend/Unavailable and holds the head of the
    /// queue until it is retried.
    Stranded,
    /// Terminal cross-tab conflict (StaleRevision on a put); never retried.
    Conflicted,
}

/// The save indicator's state, as the page names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SaveState {
    /// The editor history holds commands no put has carried yet.
    Unsaved,
    /// An operation is running, or queued behind one.
    Saving,
    /// Stored bytes match local edits.
    Saved,
    /// A path is Conflicted or Stranded; the reason names which and where.
    Failed,
}

/// A save indicator reading: the state, plus the failure reason it carries.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SaveStatus {
    pub state: SaveState,
    /// Empty but for a failure, where it names the state and the path.
    pub reason: String,
}

impl SaveStatus {
    /// The reading with no chains to consult: only the editor's own flag is left.
    fn from_history(history_dirty: bool) -> Self {
        Self {
            state: if history_dirty { SaveState::Unsaved } else { SaveState::Saved },
            reason: String::new(),
        }
    }
}

/// What the store is known to hold for one project-relative path.
pub struct PathChain {
    pub path: String,
    /// The revision the store holds, a tombstone's included; read when an operation
    /// issues, never when it is queued.
    pub base_revision: u64,
    /// A put on this path met another tab's newer revision.
    pub conflicted: bool,
}

impl PathChain {
    fn new(path: String, base_revision: u64) -> Self {
        Self {
            path,
            base_revision,
            conflicted: false,
        }
    }
}

/// Target-agnostic manager of the project's store operations.
pub struct Chains {
    project_slug: String,
    project_root: String,
    bundle_version: String,
    manifest: ProjectManifest,
    store: Arc<dyn ProjectStore>,
    chains: HashMap<String, PathChain>,
    /// Operations written but not yet issued, in write order.
    queue: VecDeque<QueuedOperation>,
    /// The one operation issued: in flight, or holding the queue as stranded or conflicted.
    head: Option<HeadOperation>,
    write_epoch: u64,
    draining: bool,
    persist_pending: Option<Arc<AtomicBool>>,
    logged_outside_paths: HashSet<String>,
}

impl Chains {
    /// Create a new chains coordinator.
    pub fn new(
        project_slug: String,
        project_root: String,
        bundle_version: String,
        manifest: ProjectManifest,
        store: Arc<dyn ProjectStore>,
        persist_pending: Option<Arc<AtomicBool>>,
    ) -> Self {
        Self {
            project_slug,
            project_root,
            bundle_version,
            manifest,
            store,
            chains: HashMap::new(),
            queue: VecDeque::new(),
            head: None,
            write_epoch: 0,
            draining: false,
            persist_pending,
            logged_outside_paths: HashSet::new(),
        }
    }

    /// Current write epoch.
    pub fn write_epoch(&self) -> u64 {
        self.write_epoch
    }

    /// Record initial base revisions from loaded project files at boot, a tombstone's
    /// included, so a put recreating a removed file names the revision it must.
    pub fn seed(&mut self, files: &[StoredFile]) {
        for file in files {
            let chain = self.chains.entry(file.path.clone()).or_insert_with(|| {
                PathChain::new(file.path.clone(), file.revision)
            });
            chain.base_revision = file.revision;
            chain.conflicted = false;
        }
        self.sync_pending_flag();
    }

    /// An operation is queued, in flight, or holding the queue as stranded or conflicted.
    pub fn is_pending(&self) -> bool {
        self.head.is_some() || !self.queue.is_empty()
    }

            /// The head operation is running in the store — what a drain awaits. The queue only
    /// ever stands behind a head, so this covers everything written before the drain. A
    /// stranded or conflicted head is not active: the drain then refuses the switch for a
    /// stranded one (unsaved work it would navigate away from) and lets a conflicted one go
    /// (the reload is what its banner asks for).
    pub fn has_active(&self) -> bool {
        self.head
            .as_ref()
            .is_some_and(|head| head.status == HeadStatus::InFlight)
    }

    /// The page's save indicator over the chains, combined with the editor's own
    /// dirty flag — which persist does not own, because edits reach persist only once
    /// a write is issued, and the page must not call those edits saved in between.
    ///
    /// A conflicted path outranks a stranded one: a conflict is terminal, while a
    /// stranded path is retried — on `visibilitychange`, on the next write, and so by
    /// the page's Save button, which the reason names.
    pub fn save_status(&self, history_dirty: bool) -> SaveStatus {
        if let Some(path) = self.conflicted_paths().first() {
            return SaveStatus {
                state: SaveState::Failed,
                reason: format!("conflicted: {path}"),
            };
        }
        if let Some(path) = self.stranded_paths().first() {
            return SaveStatus {
                state: SaveState::Failed,
                reason: format!("stranded: {path}; press Save to retry"),
            };
        }
        if self.has_active() {
            return SaveStatus { state: SaveState::Saving, reason: String::new() };
        }
        SaveStatus::from_history(history_dirty)
    }

    /// Get current state of a relative path.
    pub fn path_state(&self, relative_path: &str) -> PathState {
        if self.chains.get(relative_path).is_some_and(|chain| chain.conflicted) {
            return PathState::Conflicted;
        }
        let waiting = self.queue.iter().any(|queued| queued.path == relative_path);
        match &self.head {
            Some(head) if head.path == relative_path => match head.status {
                HeadStatus::Stranded => PathState::Stranded,
                HeadStatus::Conflicted => PathState::Conflicted,
                HeadStatus::InFlight if waiting => PathState::Queued,
                HeadStatus::InFlight => PathState::InFlight,
            },
            _ if waiting => PathState::Queued,
            _ => PathState::Idle,
        }
    }

    /// Base revision of a path.
    pub fn base_revision(&self, relative_path: &str) -> u64 {
        self.chains.get(relative_path).map(|chain| chain.base_revision).unwrap_or(0)
    }

    /// Synchronize the shared atomic pending flag with `is_pending()`.
    pub fn sync_pending_flag(&self) {
        if let Some(flag) = &self.persist_pending {
            flag.store(self.is_pending(), Ordering::Relaxed);
        }
    }

    /// Return all paths currently in Conflicted state, sorted.
    pub fn conflicted_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self
            .chains
            .values()
            .filter(|chain| chain.conflicted)
            .map(|chain| chain.path.clone())
            .collect();
        paths.sort();
        paths
    }

    /// The path whose failed operation holds the queue, if one does.
    pub fn stranded_paths(&self) -> Vec<String> {
        self.head
            .iter()
            .filter(|head| head.status == HeadStatus::Stranded)
            .map(|head| head.path.clone())
            .collect()
    }

        /// Start a project transition: bump the write epoch and mark draining, so no new
    /// write is taken while everything written before it finishes. A stranded head is
    /// retried once first — the switch is the visitor acting, evidence the tab is alive,
    /// as a write is — and if it fails again it does not block the switch.
    pub fn start_drain(&mut self) -> u64 {
        self.retry_stranded_head();
        self.write_epoch += 1;
        self.draining = true;
        self.write_epoch
    }

        /// Restore write epoch and clear draining on drain failure/timeout, so writes are
    /// taken again. On wasm the caller drives the head after it (`drive_started_operations`).
    pub fn restore_epoch(&mut self) {
        if self.write_epoch > 0 {
            self.write_epoch -= 1;
        }
        self.draining = false;
        self.issue_next();
        self.sync_pending_flag();
    }
}

/// Update text content of the DOM `#playground-banner` element, if present.
pub fn set_dom_banner(text: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(element) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id("playground-banner"))
        {
            element.set_text_content(Some(text));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = text;
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::Event;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static ACTIVE_CHAINS: std::cell::RefCell<Option<Chains>> = const { std::cell::RefCell::new(None) };
}

/// Set global active chains on wasm.
#[cfg(target_arch = "wasm32")]
pub fn set_active_chains(chains: Chains) {
    ACTIVE_CHAINS.with(|cell| *cell.borrow_mut() = Some(chains));
}

/// Access or modify global active chains on wasm.
#[cfg(target_arch = "wasm32")]
pub fn with_active_chains<R>(function: impl FnOnce(&mut Chains) -> R) -> Option<R> {
    ACTIVE_CHAINS.with(|cell| cell.borrow_mut().as_mut().map(function))
}

/// Check if any path is pending across global chains on wasm.
pub fn is_pending() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        with_active_chains(|chains| chains.is_pending()).unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// The save indicator across the global active chains on wasm.
///
/// With no chains installed — natively, and on wasm before the store opens — there is
/// no chain state to read, so the editor's own flag is the whole answer.
pub fn save_status(history_dirty: bool) -> SaveStatus {
    #[cfg(target_arch = "wasm32")]
    {
        with_active_chains(|chains| chains.save_status(history_dirty))
            .unwrap_or_else(|| SaveStatus::from_history(history_dirty))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SaveStatus::from_history(history_dirty)
    }
}

/// Drain active chains with a 5-second timeout on wasm.
#[cfg(target_arch = "wasm32")]
pub async fn drain_then_epoch() -> Result<(), String> {
            with_active_chains(|chains| chains.start_drain());
    // The drain's retry of a stranded head is a started operation nothing else drives: the
    // driver that ran it exited on the failure, and no write comes during a drain.
    drive_started_operations();

    // Await completion up to 5 seconds
    let start_time = common::clock::Instant::now();
    loop {
        let (active, stranded) = with_active_chains(|chains| {
            (chains.has_active(), chains.stranded_paths().into_iter().next())
        })
        .unwrap_or((false, None));
        if !active {
            // A head stranded under the drain — its one retry failed too — holds unsaved
            // work and everything queued behind it; the page would navigate away from both.
            if let Some(path) = stranded {
                with_active_chains(|chains| chains.restore_epoch());
                drive_started_operations();
                return Err(format!(
                    "{path} is not saved to this browser — export your project, then try again"
                ));
            }
            return Ok(());
        }
        if start_time.elapsed() > std::time::Duration::from_secs(5) {
                        with_active_chains(|chains| {
                chains.restore_epoch();
            });
            drive_started_operations();
            return Err("operation timed out waiting for pending saves; tab visibility is the usual cause — try again with the tab in foreground".to_string());
        }

        // Yield to event loop
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            if let Some(window) = web_sys::window() {
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 50);
            }
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }
}

/// Install window event listeners (`visibilitychange`, `beforeunload`).
#[cfg(target_arch = "wasm32")]
pub fn install_listeners() {
    let window = match web_sys::window() {
        Some(window) => window,
        None => return,
    };

    // visibilitychange -> hidden: retry a stranded head
    if let Some(document) = window.document() {
        let on_visibility_change = Closure::wrap(Box::new(move |_event: Event| {
            if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                if document.visibility_state() == web_sys::VisibilityState::Hidden {
                    with_active_chains(|chains| {
                        chains.reissue_stranded();
                    });
                    drive_started_operations();
                }
            }
        }) as Box<dyn FnMut(Event)>);
        let _ = document.add_event_listener_with_callback("visibilitychange", on_visibility_change.as_ref().unchecked_ref());
        on_visibility_change.forget();
    }

    // beforeunload: warn while anything is pending
    let on_beforeunload = Closure::wrap(Box::new(move |event: web_sys::BeforeUnloadEvent| {
        if is_pending() {
            event.set_return_value("Changes you made may not be saved.");
        }
    }) as Box<dyn FnMut(web_sys::BeforeUnloadEvent)>);
    let _ = window.add_event_listener_with_callback("beforeunload", on_beforeunload.as_ref().unchecked_ref());
    on_beforeunload.forget();
}

/// Drive the started store operation on the wasm single-threaded event loop; its
/// completion issues the next, which this drives in turn.
#[cfg(target_arch = "wasm32")]
pub fn drive_started_operations() {
    let Some((path, future)) = with_active_chains(|chains| chains.take_started_operation()).flatten()
    else {
        return;
    };
    wasm_bindgen_futures::spawn_local(async move {
        let result = future.await;
        with_active_chains(|chains| chains.handle_completion(&path, result));
        drive_started_operations();
    });
}

#[cfg(test)]
mod tests;
