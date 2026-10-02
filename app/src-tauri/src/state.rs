//! What the app keeps while it runs: the write lock, what it wrote itself,
//! and the open project's folder watcher.

use std::sync::{Arc, Mutex, MutexGuard};

use crate::watch::{OwnWrites, Watching};

#[derive(Default)]
pub struct AppState {
    /// Serializes writes so two saves never read-modify-write the same file at once.
    lock: Mutex<()>,
    /// What the app itself wrote, to tell it from other devices' changes.
    pub own: Arc<OwnWrites>,
    /// The open project's folder watcher.
    watching: Mutex<Option<Watching>>,
}

impl AppState {
    pub fn write(&self) -> MutexGuard<'_, ()> {
        self.lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The lock itself, for work that takes it around each write (sync passes).
    pub fn lock(&self) -> &Mutex<()> {
        &self.lock
    }

    pub fn watch(&self, watching: Option<Watching>) {
        let mut slot = self
            .watching
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = watching;
    }

    /// Stops watching `root`, unless another project is watched by now.
    pub fn unwatch(&self, root: &str) {
        let mut slot = self
            .watching
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if slot.as_ref().is_some_and(|w| w.root == root) {
            *slot = None;
        }
    }
}
