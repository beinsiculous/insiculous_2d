//! Write persistence state machine for the playground.
//!
//! Provides per-path compare-and-swap chaining, write-epoch gating,
//! and dirty/banner synchronisation.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::projects::ProjectManifest;
use crate::store::{Fut, ProjectStore, StoredFile, StoreError};

/// States a file path can occupy in the persistence state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathState {
    /// Stored bytes match local edits.
    Idle,
    /// A put is currently running.
    InFlight,
    /// A put is running and newer bytes are queued behind it.
    Queued,
    /// Last put failed with Backend/Unavailable; holds newest bytes.
    Stranded,
    /// Terminal cross-tab conflict (StaleRevision); never retried.
    Conflicted,
}

/// State tracking for a single project-relative file path.
pub struct PathChain {
    pub path: String,
    pub base_revision: u64,
    pub state: PathState,
    pub in_flight_bytes: Option<Vec<u8>>,
    pub queued_bytes: Option<Vec<u8>>,
    pub stranded_bytes: Option<Vec<u8>>,
    pub in_flight_future: Option<Fut<Result<u64, StoreError>>>,
}

impl PathChain {
    fn new(path: String, base_revision: u64) -> Self {
        Self {
            path,
            base_revision,
            state: PathState::Idle,
            in_flight_bytes: None,
            queued_bytes: None,
            stranded_bytes: None,
            in_flight_future: None,
        }
    }
}

/// Target-agnostic manager of per-path write chains.
pub struct Chains {
    project_slug: String,
    project_root: String,
    bundle_version: String,
    manifest: ProjectManifest,
    store: Arc<dyn ProjectStore>,
    chains: HashMap<String, PathChain>,
    write_epoch: u64,
    draining: bool,
    persist_pending: Option<Arc<AtomicBool>>,
    logged_outside_paths: std::collections::HashSet<String>,
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
            write_epoch: 0,
            draining: false,
            persist_pending,
            logged_outside_paths: std::collections::HashSet::new(),
        }
    }

    /// Current write epoch.
    pub fn write_epoch(&self) -> u64 {
        self.write_epoch
    }

    /// Record initial base revisions from loaded project files at boot.
    pub fn seed(&mut self, files: &[StoredFile]) {
        for file in files {
            let chain = self.chains.entry(file.path.clone()).or_insert_with(|| {
                PathChain::new(file.path.clone(), file.revision)
            });
            chain.base_revision = file.revision;
            chain.state = PathState::Idle;
        }
        self.sync_pending_flag();
    }

    /// Any path not in the Idle state.
    pub fn is_pending(&self) -> bool {
        self.chains.values().any(|chain| chain.state != PathState::Idle)
    }

    /// Any path with an active put running or queued.
    pub fn has_active(&self) -> bool {
        self.chains.values().any(|chain| {
            chain.state == PathState::InFlight || chain.state == PathState::Queued
        })
    }

    /// Get current state of a relative path.
    pub fn path_state(&self, relative_path: &str) -> PathState {
        self.chains.get(relative_path).map(|chain| chain.state).unwrap_or(PathState::Idle)
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

    /// Extract all started in-flight futures, leaving chain state untouched.
    pub fn take_started_puts(&mut self) -> Vec<(String, Fut<Result<u64, StoreError>>)> {
        let mut started_puts = Vec::new();
        for (path, chain) in &mut self.chains {
            if let Some(future) = chain.in_flight_future.take() {
                started_puts.push((path.clone(), future));
            }
        }
        started_puts
    }

    /// Handle a VFS write notification.
    pub fn on_vfs_write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        if self.draining {
            return Err("project is being replaced — save again after the reload".to_string());
        }

        let path_string = path.to_string_lossy();
        let normalized_root = self.project_root.trim_end_matches('/');
        // The root is a directory boundary: "/projects/pong2/x" must not strip to "2/x".
        let relative_path = match path_string.strip_prefix(normalized_root) {
            Some(stripped) if stripped.is_empty() || stripped.starts_with('/') => {
                stripped.trim_start_matches('/')
            }
            _ => {
                if self.logged_outside_paths.insert(path_string.into_owned()) {
                    log::warn!("vfs write at '{}' outside project root '{}' ignored for persistence", path.display(), normalized_root);
                    set_dom_banner(&format!(
                        "{} is outside the open project and is not saved to this browser — save under the project, or export",
                        path.display()
                    ));
                }
                return Ok(());
            }
        };

        let chain = self.chains.entry(relative_path.to_string()).or_insert_with(|| {
            PathChain::new(relative_path.to_string(), 0)
        });

        match chain.state {
            PathState::Conflicted => {
                // Do not issue put; MemFs holds newest bytes for export.
            }
            PathState::InFlight | PathState::Queued => {
                chain.queued_bytes = Some(bytes.to_vec());
                chain.state = PathState::Queued;
            }
            PathState::Idle | PathState::Stranded => {
                chain.state = PathState::InFlight;
                chain.in_flight_bytes = Some(bytes.to_vec());
                chain.queued_bytes = None;
                chain.stranded_bytes = None;

                let file = StoredFile {
                    project: self.project_slug.clone(),
                    path: relative_path.to_string(),
                    bytes: bytes.to_vec(),
                    revision: 0,
                    bundle_version: self.bundle_version.clone(),
                };
                let future = self.store.put(file, chain.base_revision, &self.manifest);
                chain.in_flight_future = Some(future);
            }
        }

        self.sync_pending_flag();
        Ok(())
    }

    /// Poll all in-flight futures once, handling completions.
    pub fn poll_all(&mut self, cx: &mut Context<'_>) -> bool {
        let mut any_completed = false;
        let mut completions = Vec::new();

        for (path, chain) in &mut self.chains {
            if let Some(future) = &mut chain.in_flight_future {
                if let Poll::Ready(result) = future.as_mut().poll(cx) {
                    completions.push((path.clone(), result));
                }
            }
        }

        for (path, result) in completions {
            any_completed = true;
            self.handle_put_completion(&path, result);
        }

        if any_completed {
            self.sync_pending_flag();
        }

        any_completed
    }

    /// Process completion of a put operation for `path`.
    pub fn handle_put_completion(&mut self, path: &str, result: Result<u64, StoreError>) {
        let chain = match self.chains.get_mut(path) {
            Some(chain) => chain,
            None => return,
        };
        chain.in_flight_future = None;

        match result {
            Ok(new_revision) => {
                chain.base_revision = new_revision;
                if let Some(queued) = chain.queued_bytes.take() {
                    chain.state = PathState::InFlight;
                    chain.in_flight_bytes = Some(queued.clone());
                    let file = StoredFile {
                        project: self.project_slug.clone(),
                        path: path.to_string(),
                        bytes: queued,
                        revision: 0,
                        bundle_version: self.bundle_version.clone(),
                    };
                    let future = self.store.put(file, chain.base_revision, &self.manifest);
                    chain.in_flight_future = Some(future);
                } else {
                    chain.state = PathState::Idle;
                    chain.in_flight_bytes = None;
                }
            }
            Err(StoreError::StaleRevision { stored, base: _ }) => {
                chain.state = PathState::Conflicted;
                chain.queued_bytes = None;
                chain.in_flight_bytes = None;
                if stored > chain.base_revision {
                    chain.base_revision = stored;
                }
                set_dom_banner(&format!(
                    "another tab saved {path} after you loaded it; reloading discards THIS tab's version — export first to keep it"
                ));
            }
            Err(StoreError::Unavailable) | Err(StoreError::Backend(_)) => {
                if let Some(queued) = chain.queued_bytes.take() {
                    chain.state = PathState::InFlight;
                    chain.in_flight_bytes = Some(queued.clone());
                    let file = StoredFile {
                        project: self.project_slug.clone(),
                        path: path.to_string(),
                        bytes: queued,
                        revision: 0,
                        bundle_version: self.bundle_version.clone(),
                    };
                    let future = self.store.put(file, chain.base_revision, &self.manifest);
                    chain.in_flight_future = Some(future);
                } else {
                    chain.state = PathState::Stranded;
                    chain.stranded_bytes = chain.in_flight_bytes.take();
                    set_dom_banner("not saved to this browser — export your project");
                }
            }
        }
        self.sync_pending_flag();
    }

    /// Re-issue put for stranded paths (triggered on `visibilitychange` -> hidden).
    pub fn reissue_stranded(&mut self) {
        let stranded_paths: Vec<(String, Vec<u8>)> = self
            .chains
            .iter()
            .filter_map(|(path, chain)| {
                if chain.state == PathState::Stranded {
                    chain.stranded_bytes.clone().map(|bytes| (path.clone(), bytes))
                } else {
                    None
                }
            })
            .collect();

        for (path, bytes) in stranded_paths {
            if let Some(chain) = self.chains.get_mut(&path) {
                chain.state = PathState::InFlight;
                chain.in_flight_bytes = Some(bytes.clone());
                chain.stranded_bytes = None;
                let file = StoredFile {
                    project: self.project_slug.clone(),
                    path: path.clone(),
                    bytes,
                    revision: 0,
                    bundle_version: self.bundle_version.clone(),
                };
                let future = self.store.put(file, chain.base_revision, &self.manifest);
                chain.in_flight_future = Some(future);
            }
        }

        self.sync_pending_flag();
    }

    /// Start a project transition: bump write epoch, mark draining, then report whether active puts remain.
    pub fn start_drain(&mut self) -> u64 {
        self.write_epoch += 1;
        self.draining = true;
        self.write_epoch
    }

    /// Restore write epoch and clear draining on drain failure/timeout.
    pub fn restore_epoch(&mut self) {
        if self.write_epoch > 0 {
            self.write_epoch -= 1;
        }
        self.draining = false;
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

/// Drain active chains with a 5-second timeout on wasm.
#[cfg(target_arch = "wasm32")]
pub async fn drain_then_epoch() -> Result<(), String> {
    let has_active = with_active_chains(|chains| {
        chains.start_drain();
        chains.has_active()
    }).unwrap_or(false);

    if !has_active {
        return Ok(());
    }

    // Await completion up to 5 seconds
    let start_time = common::clock::Instant::now();
    loop {
        let active = with_active_chains(|chains| chains.has_active()).unwrap_or(false);
        if !active {
            return Ok(());
        }
        if start_time.elapsed() > std::time::Duration::from_secs(5) {
            with_active_chains(|chains| {
                chains.restore_epoch();
            });
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

    // 1. visibilitychange -> hidden: reissue stranded
    if let Some(document) = window.document() {
        let on_visibility_change = Closure::wrap(Box::new(move |_event: Event| {
            if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                if document.visibility_state() == web_sys::VisibilityState::Hidden {
                    with_active_chains(|chains| {
                        chains.reissue_stranded();
                    });
                    drive_started_puts();
                }
            }
        }) as Box<dyn FnMut(Event)>);
        let _ = document.add_event_listener_with_callback("visibilitychange", on_visibility_change.as_ref().unchecked_ref());
        on_visibility_change.forget();
    }

    // 2. beforeunload: set warning if is_pending()
    let on_beforeunload = Closure::wrap(Box::new(move |event: web_sys::BeforeUnloadEvent| {
        if is_pending() {
            event.set_return_value("Changes you made may not be saved.");
        }
    }) as Box<dyn FnMut(web_sys::BeforeUnloadEvent)>);
    let _ = window.add_event_listener_with_callback("beforeunload", on_beforeunload.as_ref().unchecked_ref());
    on_beforeunload.forget();
}

/// Drive started put futures on the wasm single-threaded event loop.
#[cfg(target_arch = "wasm32")]
pub fn drive_started_puts() {
    let started_puts = with_active_chains(|chains| chains.take_started_puts()).unwrap_or_default();
    for (path, future) in started_puts {
        wasm_bindgen_futures::spawn_local(async move {
            let result = future.await;
            with_active_chains(|chains| {
                chains.handle_put_completion(&path, result);
                chains.sync_pending_flag();
            });
            drive_started_puts();
        });
    }
}

#[cfg(test)]
mod tests;
