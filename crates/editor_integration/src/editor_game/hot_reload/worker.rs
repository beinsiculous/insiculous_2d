//! The thread that owns the editor's [`FileWatch`] and polls it, so no file is ever read
//! for its stamp on the frame. It takes two messages — the paths to watch with those the
//! editor expects missing, and a path the editor wrote with the stamp it read after — and
//! sends back what settled and, for each write, that it took the note. On the web the same type starts nothing, takes nothing and
//! reports nothing.
//!
//! [`FileWatch`]: editor::external_tools::FileWatch

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::editor_game) use native::WatchWorker;
#[cfg(target_arch = "wasm32")]
pub(in crate::editor_game) use web::WatchWorker;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::collections::BTreeSet;
    use std::path::PathBuf;
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
    use std::thread::JoinHandle;
    use std::time::Duration;

    use common::clock::Instant;
    use common::vfs::ChangeStamp;
    use editor::external_tools::{FileWatch, WatchEvent};

    enum WatchMessage {
        /// The paths to watch, and those among them the editor expects missing.
        Register(BTreeSet<PathBuf>, BTreeSet<PathBuf>),
        Written(PathBuf, Option<ChangeStamp>),
    }

    /// The handle the frame holds: messages out, events back. Dropping it hangs up on the
    /// thread and joins it.
    pub(in crate::editor_game) struct WatchWorker {
        messages: Option<Sender<WatchMessage>>,
        events: Receiver<WatchEvent>,
        thread: Option<JoinHandle<()>>,
    }

    impl WatchWorker {
        /// Start polling every `interval`, watching nothing until [`register`](Self::register).
        pub(in crate::editor_game) fn spawn(interval: Duration) -> Self {
            let (message_sender, message_receiver) = mpsc::channel();
            let (event_sender, event_receiver) = mpsc::channel();
            let spawned = std::thread::Builder::new()
                .name("file-watch".to_string())
                .spawn(move || run(&message_receiver, &event_sender, interval));
            let thread = match spawned {
                Ok(thread) => Some(thread),
                Err(error) => {
                    log::error!("the file watcher did not start, so no file reloads on its own: {error}");
                    None
                }
            };
            Self { messages: Some(message_sender), events: event_receiver, thread }
        }

        /// Watch exactly `paths` from the next message on, those in `expected_absent` taken as
        /// missing until the first poll reads them (`FileWatch::set_registered`).
        pub(in crate::editor_game) fn register(&self, paths: BTreeSet<PathBuf>, expected_absent: BTreeSet<PathBuf>) {
            self.send(WatchMessage::Register(paths, expected_absent));
        }

        /// The editor wrote `path` itself and read `stamp` right after, `None` where the
        /// filesystem keeps none. Answered with [`WatchEvent::Noted`] either way.
        pub(in crate::editor_game) fn note_written(&self, path: PathBuf, stamp: Option<ChangeStamp>) {
            self.send(WatchMessage::Written(path, stamp));
        }

        /// Every event the thread has sent since the last call, in order, never blocking.
        pub(in crate::editor_game) fn take_events(&self) -> Vec<WatchEvent> {
            self.events.try_iter().collect()
        }

        fn send(&self, message: WatchMessage) {
            // A thread that never started, or one gone, has nobody to tell.
            if let Some(sender) = &self.messages {
                sender.send(message).ok();
            }
        }
    }

    impl Drop for WatchWorker {
        fn drop(&mut self) {
            self.messages = None;
            if let Some(thread) = self.thread.take() {
                if thread.join().is_err() {
                    log::error!("the file watcher's thread panicked");
                }
            }
        }
    }

    /// Apply each message as it comes and poll whenever `interval` has passed, until the
    /// frame's handle hangs up. A message never delays a poll past its time.
    fn run(messages: &Receiver<WatchMessage>, events: &Sender<WatchEvent>, interval: Duration) {
        let mut watch = FileWatch::new();
        let mut next_poll = Instant::now() + interval;
        loop {
            let wait = next_poll.saturating_duration_since(Instant::now());
            let sent = match messages.recv_timeout(wait) {
                Ok(WatchMessage::Register(paths, expected_absent)) => {
                    watch.set_registered(paths, &expected_absent);
                    Ok(())
                }
                Ok(WatchMessage::Written(path, stamp)) => {
                    if let Some(stamp) = stamp {
                        watch.note_written(&path, stamp);
                    }
                    events.send(WatchEvent::Noted(path))
                }
                Err(RecvTimeoutError::Timeout) => {
                    let sent = watch.poll().into_iter().try_for_each(|change| events.send(WatchEvent::Change(change)));
                    next_poll = Instant::now() + interval;
                    sent
                }
                Err(RecvTimeoutError::Disconnected) => return,
            };
            if sent.is_err() {
                return;
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use std::collections::BTreeSet;
    use std::path::PathBuf;
    use std::time::Duration;

    use common::vfs::ChangeStamp;
    use editor::external_tools::WatchEvent;

    /// The native worker's surface with nothing behind it: the browser's files are the
    /// playground's, which never change under the editor.
    pub(in crate::editor_game) struct WatchWorker;

    impl WatchWorker {
        pub(in crate::editor_game) fn spawn(_interval: Duration) -> Self {
            Self
        }

        pub(in crate::editor_game) fn register(&self, _paths: BTreeSet<PathBuf>, _expected_absent: BTreeSet<PathBuf>) {}

        pub(in crate::editor_game) fn note_written(&self, _path: PathBuf, _stamp: Option<ChangeStamp>) {}

        pub(in crate::editor_game) fn take_events(&self) -> Vec<WatchEvent> {
            Vec::new()
        }
    }
}
