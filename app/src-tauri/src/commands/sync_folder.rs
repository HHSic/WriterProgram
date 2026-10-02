//! Other devices through a sync folder: copies left by sync programs, where
//! projects are kept, and watching the open project for changes that arrive.

use std::path::Path;

use tauri::{AppHandle, Manager, State};
use writer_core::copies::{self, Resolve};
use writer_core::doc::Section;
use writer_core::places::{self, Env, Place};

use super::doc::DocData;
use crate::error::{Res, fail};
use crate::state::AppState;
use crate::watch;

/// A copy of a chapter or planning document, to compare with the original.
#[tauri::command]
pub async fn copy_load(root: String, section: Section, file: String) -> Res<DocData> {
    copies::load(Path::new(&root), section, &file)
        .map(DocData::from)
        .map_err(fail)
}

#[tauri::command]
pub async fn copy_resolve(
    state: State<'_, AppState>,
    root: String,
    section: Section,
    file: String,
    action: Resolve,
) -> Res<()> {
    let _write = state.write();
    copies::resolve(Path::new(&root), section, &file, action).map_err(fail)
}

fn detect_places(app: &AppHandle) -> Vec<Place> {
    places::detect(&Env::current(app.path().document_dir().ok()))
}

/// Folders that OneDrive, Google Drive, Dropbox or iCloud keep in step with
/// other devices, then this computer only.
#[tauri::command]
pub async fn storage_places(app: AppHandle) -> Res<Vec<Place>> {
    Ok(detect_places(&app))
}

/// The sync folder a path is in, if any.
#[tauri::command]
pub async fn storage_of(app: AppHandle, path: String) -> Res<Option<Place>> {
    let places = detect_places(&app);
    Ok(places::place_of(Path::new(&path), &places).cloned())
}

/// Starts telling the screen about changes in the project folder that the
/// app did not make (event "project-changed").
#[tauri::command]
pub async fn project_watch(app: AppHandle, state: State<'_, AppState>, root: String) -> Res<()> {
    state.watch(None);
    state.watch(Some(watch::start(&app, &root)?));
    Ok(())
}

#[tauri::command]
pub async fn project_unwatch(state: State<'_, AppState>, root: String) -> Res<()> {
    state.unwatch(&root);
    Ok(())
}
