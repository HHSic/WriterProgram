//! Watches the open project folder and tells the screen about changes the app
//! did not make itself: another device's edits arriving through a sync
//! program, or files changed by hand (see `writer_core::changes`).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use writer_core::changes::{self, Change};
use writer_core::copies;
use writer_core::journal;
use writer_core::store::rev_of;

use crate::state::AppState;

/// Changes settle this long before they are passed on: a sync program often
/// writes a file in several steps.
const SETTLE: Duration = Duration::from_millis(400);

/// Fingerprints of what the app itself last wrote to each file, to tell its
/// own saves from changes made by others.
#[derive(Default)]
pub struct OwnWrites(Mutex<HashMap<String, String>>);

impl OwnWrites {
    pub fn record(&self, path: &Path, bytes: &[u8]) {
        let mut map = self.0.lock().unwrap_or_else(|p| p.into_inner());
        map.insert(key(path), rev_of(bytes));
    }

    fn is_own(&self, path: &Path, bytes: &[u8]) -> bool {
        let map = self.0.lock().unwrap_or_else(|p| p.into_inner());
        map.get(&key(path)).is_some_and(|rev| *rev == rev_of(bytes))
    }
}

fn key(path: &Path) -> String {
    let s = path.to_string_lossy();
    if cfg!(windows) {
        s.replace('/', "\\").to_lowercase()
    } else {
        s.into_owned()
    }
}

/// What the screen hears: the project the changes are in, and the changes.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Batch {
    root: String,
    changes: Vec<Change>,
}

/// Watching stops when this is dropped.
pub struct Watching {
    pub root: String,
    _debouncer: Debouncer<RecommendedWatcher>,
}

pub fn start(app: &AppHandle, root: &str) -> Result<Watching, String> {
    let root_path = PathBuf::from(root);
    let root_text = root.to_string();
    let handle = app.clone();
    let mut debouncer = new_debouncer(SETTLE, move |result: DebounceEventResult| {
        let Ok(events) = result else {
            return;
        };
        let state = handle.state::<AppState>();
        let mut out: Vec<Change> = Vec::new();
        let mut merge = false;
        for event in events {
            let path = &event.path;
            let Ok(rel) = path.strip_prefix(&root_path) else {
                continue;
            };
            // The creation journal grows with every save and is nothing the
            // screen shows: not worth reading.
            if rel.starts_with(journal::JOURNAL_DIR) {
                continue;
            }
            let bytes = if path.is_dir() {
                None
            } else {
                match fs::read(path) {
                    Ok(bytes) => Some(bytes),
                    // Still being written: the next change brings it.
                    Err(_) if path.exists() => continue,
                    Err(_) => None,
                }
            };
            if let Some(bytes) = &bytes
                && state.own.is_own(path, bytes)
            {
                continue;
            }
            match changes::describe(rel, bytes.as_deref()) {
                Some(Change::ProjectCopy) => merge = true,
                Some(change) if !out.contains(&change) => out.push(change),
                _ => {}
            }
        }
        if merge {
            let _write = state.write();
            if copies::merge_project_copies(&root_path).unwrap_or(false)
                && !out.contains(&Change::Project)
            {
                out.push(Change::Project);
            }
        }
        if !out.is_empty() {
            let _ = handle.emit(
                "project-changed",
                Batch {
                    root: root_text.clone(),
                    changes: out,
                },
            );
        }
    })
    .map_err(|e| e.to_string())?;
    debouncer
        .watcher()
        .watch(Path::new(root), RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;
    Ok(Watching {
        root: root.to_string(),
        _debouncer: debouncer,
    })
}
