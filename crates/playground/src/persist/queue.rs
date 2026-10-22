//! The store's one queue: every put and every removal the project's files see, issued
//! one at a time in the order they were written.
//!
//! **Write order is the durable order.** Nothing coalesces — a path written twice is two
//! puts — so whatever the store holds when any operation completes is a prefix of the
//! write sequence: the same states a native crash leaves, which the song's open-time
//! rules were written for. A save's journal pair therefore lands before its finals, and
//! a removal after every put written ahead of it.
//!
//! **Failure is head-of-line.** An operation that fails (Backend/Unavailable) strands the
//! head and nothing behind it issues, so a save whose final failed never reaches its
//! removals. The head is retried on `visibilitychange` and on the next write, a new write
//! being evidence the tab is alive. A conflicted put holds the queue too: the tab is in
//! conflict and the banner says to export and reload.
//!
//! **A removal is a tombstone** one revision up, so a stale tab's removal is stale by
//! revision; that one is dropped with a log line, the other tab's write winning.

use std::path::Path;
use std::task::{Context, Poll};

use super::{set_dom_banner, Chains, PathChain};
use crate::store::{Fut, StoreError, StoredFile};

/// What a queued entry asks of the store.
pub(super) enum Operation {
    Put(Vec<u8>),
    Remove,
}

/// An operation written but not yet issued.
pub(super) struct QueuedOperation {
    pub(super) path: String,
    operation: Operation,
}

/// Where the issued operation stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HeadStatus {
    InFlight,
    /// Failed with Backend/Unavailable; kept to be issued again.
    Stranded,
    /// A put met another tab's newer revision; never issued again.
    Conflicted,
}

/// The one operation issued.
pub(super) struct HeadOperation {
    pub(super) path: String,
    operation: Operation,
    /// The running store call; taken by the wasm driver once it spawns it.
    future: Option<Fut<Result<u64, StoreError>>>,
    pub(super) status: HeadStatus,
}

impl Chains {
        /// Handle a VFS write notification: queue a put of `bytes` behind everything written
    /// before it, its own path's removal included — a removal cancelled would reorder the
    /// store's states against the write sequence, and the put names the tombstone's
    /// revision when it issues.
    pub fn on_vfs_write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        if self.draining {
            return Err("project is being replaced — save again after the reload".to_string());
        }
        let Some(relative_path) = self.project_relative_path(path, true) else {
            return Ok(());
        };
        let chain = self
            .chains
            .entry(relative_path.clone())
            .or_insert_with(|| PathChain::new(relative_path.clone(), 0));
        if chain.conflicted {
            // No put is issued; MemFs holds the newest bytes for export.
            return Ok(());
        }
                self.enqueue(relative_path, Operation::Put(bytes.to_vec()));
        Ok(())
    }

    /// Handle a VFS removal notification: queue a tombstone for a path the store may
    /// hold. A path never loaded nor written in this tab has no chain and nothing stored.
    pub fn on_vfs_remove(&mut self, path: &Path) {
        if self.draining {
            return;
        }
        let Some(relative_path) = self.project_relative_path(path, false) else {
            return;
        };
        if self
            .chains
            .get(&relative_path)
            .is_none_or(|chain| chain.conflicted)
        {
            return;
        }
        self.enqueue(relative_path, Operation::Remove);
    }

    /// Poll the head operation once, handling its completion. `true` when it completed.
    pub fn poll_head(&mut self, context: &mut Context<'_>) -> bool {
        let Some(head) = self.head.as_mut() else {
            return false;
        };
        let Some(future) = head.future.as_mut() else {
            return false;
        };
        let Poll::Ready(result) = future.as_mut().poll(context) else {
            return false;
        };
        let path = head.path.clone();
        self.handle_completion(&path, result);
        true
    }

    /// Take the head's running store call for a driver to await, leaving it in flight.
    pub fn take_started_operation(&mut self) -> Option<(String, Fut<Result<u64, StoreError>>)> {
        let head = self.head.as_mut()?;
        let future = head.future.take()?;
        Some((head.path.clone(), future))
    }

    /// Process the head operation's result: the revision the store now holds for its
    /// path, or why it holds nothing new.
    pub fn handle_completion(&mut self, path: &str, result: Result<u64, StoreError>) {
        let Some(head) = self
            .head
            .as_mut()
            .filter(|head| head.path == path && head.status == HeadStatus::InFlight)
        else {
            log::warn!("a store result for '{path}' arrived with no operation of it in flight; ignored");
            return;
        };
        head.future = None;
        let is_removal = matches!(head.operation, Operation::Remove);
        match result {
            Ok(revision) => {
                self.head = None;
                if let Some(chain) = self.chains.get_mut(path) {
                    chain.base_revision = revision;
                }
                self.issue_next();
            }
            Err(StoreError::StaleRevision { stored, .. }) if is_removal => {
                log::info!(
                    "{path} was rewritten at revision {stored} by another tab; this tab's removal of it is dropped"
                );
                self.head = None;
                self.issue_next();
            }
            Err(StoreError::StaleRevision { stored, .. }) => {
                head.status = HeadStatus::Conflicted;
                if let Some(chain) = self.chains.get_mut(path) {
                    chain.conflicted = true;
                    chain.base_revision = chain.base_revision.max(stored);
                }
                set_dom_banner(&format!(
                    "another tab saved {path} after you loaded it; reloading discards THIS tab's version — export first to keep it"
                ));
            }
            Err(StoreError::Unavailable) | Err(StoreError::Backend(_)) => {
                head.status = HeadStatus::Stranded;
                set_dom_banner("not saved to this browser — export your project");
            }
        }
        self.sync_pending_flag();
    }

    /// Issue a stranded head again (triggered on `visibilitychange` -> hidden).
    pub fn reissue_stranded(&mut self) {
        self.retry_stranded_head();
        self.sync_pending_flag();
    }

    fn enqueue(&mut self, path: String, operation: Operation) {
        self.queue.push_back(QueuedOperation { path, operation });
        self.retry_stranded_head();
        self.issue_next();
        self.sync_pending_flag();
    }

        /// Issue the first queued operation when nothing holds the queue. A drain does not
    /// hold it: what was written before the drain is what the drain waits for.
    pub(super) fn issue_next(&mut self) {
        if self.head.is_some() {
            return;
        }
        let Some(next) = self.queue.pop_front() else {
            return;
        };
        let future = self.start_operation(&next.path, &next.operation);
        self.head = Some(HeadOperation {
            path: next.path,
            operation: next.operation,
            future: Some(future),
            status: HeadStatus::InFlight,
        });
    }

    pub(super) fn retry_stranded_head(&mut self) {
        if self.draining {
            return;
        }
        let Some(head) = self
            .head
            .as_ref()
            .filter(|head| head.status == HeadStatus::Stranded)
        else {
            return;
        };
        let future = self.start_operation(&head.path, &head.operation);
        if let Some(head) = self.head.as_mut() {
            head.future = Some(future);
            head.status = HeadStatus::InFlight;
        }
    }

    /// Call the store for `operation`, with the base revision as it stands now: an
    /// operation ahead of this one on the same path may have moved it since it was queued.
    fn start_operation(&self, path: &str, operation: &Operation) -> Fut<Result<u64, StoreError>> {
        let base_revision = self.base_revision(path);
        match operation {
            Operation::Put(bytes) => {
                let file = StoredFile {
                    project: self.project_slug.clone(),
                    path: path.to_string(),
                    bytes: bytes.clone(),
                    revision: 0,
                    bundle_version: self.bundle_version.clone(),
                    deleted: false,
                };
                self.store.put(file, base_revision, &self.manifest)
            }
            Operation::Remove => {
                let removal = self.store.remove_file(&self.project_slug, path, base_revision);
                // At revision zero the store held nothing, so nothing was tombstoned.
                let revision_after = if base_revision == 0 { 0 } else { base_revision + 1 };
                Box::pin(async move { removal.await.map(|()| revision_after) })
            }
        }
    }

    /// `path` relative to the project root, or `None` outside it. A write outside the
    /// root is not saved here, and the first one at each path says so on the banner.
    fn project_relative_path(&mut self, path: &Path, announce_outside: bool) -> Option<String> {
        let path_string = path.to_string_lossy();
        let normalized_root = self.project_root.trim_end_matches('/');
        // The root is a directory boundary: "/projects/pong2/x" must not strip to "2/x".
        match path_string.strip_prefix(normalized_root) {
            Some(stripped) if stripped.is_empty() || stripped.starts_with('/') => {
                Some(stripped.trim_start_matches('/').to_string())
            }
            _ => {
                if announce_outside && self.logged_outside_paths.insert(path_string.to_string()) {
                    log::warn!(
                        "vfs write at '{}' outside project root '{}' ignored for persistence",
                        path.display(),
                        normalized_root
                    );
                    set_dom_banner(&format!(
                        "{} is outside the open project and is not saved to this browser — save under the project, or export",
                        path.display()
                    ));
                }
                None
            }
        }
    }
}
