//! Projects: the recent list, making, opening, changing and moving a
//! project, its parts, and showing files in the file manager.

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;
use writer_core::project::{self, NewProject, Overview, ProjectInfo, ProjectPatch};
use writer_core::recent::{self, RecentItem};

use crate::error::{Res, fail};
use crate::paths::recent_file;
use crate::state::AppState;

#[tauri::command]
pub async fn recent_list(app: AppHandle) -> Res<Vec<RecentItem>> {
    Ok(recent::list(&recent_file(&app)?))
}

#[tauri::command]
pub async fn recent_remove(app: AppHandle, path: String) -> Res<()> {
    recent::remove(&recent_file(&app)?, &path).map_err(fail)
}

/// Suggested place for new projects: Documents/WriterProgram.
#[tauri::command]
pub async fn default_location(app: AppHandle) -> Res<String> {
    let docs = app.path().document_dir().map_err(|e| e.to_string())?;
    Ok(docs.join("WriterProgram").to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn project_create(
    app: AppHandle,
    state: State<'_, AppState>,
    opts: NewProject,
) -> Res<Overview> {
    let _write = state.write();
    let root = project::create(&opts).map_err(fail)?;
    let overview = project::open(&root).map_err(fail)?;
    let _ = recent::touch(&recent_file(&app)?, &overview);
    Ok(overview)
}

#[tauri::command]
pub async fn project_open(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Res<Overview> {
    let _write = state.write();
    let overview = project::open(Path::new(&path)).map_err(fail)?;
    let _ = recent::touch(&recent_file(&app)?, &overview);
    Ok(overview)
}

#[tauri::command]
pub async fn project_overview(root: String) -> Res<Overview> {
    project::overview(Path::new(&root)).map_err(fail)
}

#[tauri::command]
pub async fn project_update(
    state: State<'_, AppState>,
    root: String,
    patch: ProjectPatch,
) -> Res<ProjectInfo> {
    let _write = state.write();
    project::update(Path::new(&root), &patch).map_err(fail)
}

#[tauri::command]
pub async fn part_add(state: State<'_, AppState>, root: String, title: String) -> Res<String> {
    let _write = state.write();
    project::add_part(Path::new(&root), &title).map_err(fail)
}

#[tauri::command]
pub async fn part_rename(
    state: State<'_, AppState>,
    root: String,
    part_id: String,
    title: String,
) -> Res<()> {
    let _write = state.write();
    project::rename_part(Path::new(&root), &part_id, &title).map_err(fail)
}

#[tauri::command]
pub async fn part_remove(state: State<'_, AppState>, root: String, part_id: String) -> Res<()> {
    let _write = state.write();
    project::remove_part(Path::new(&root), &part_id).map_err(fail)
}

/// Shows a file or folder in the system file manager.
#[tauri::command]
pub async fn reveal(app: AppHandle, path: String) -> Res<()> {
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveOutcome {
    overview: Overview,
    left_behind: bool,
}

/// Moves the project folder into `dest` and opens it there.
#[tauri::command]
pub async fn project_move(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
    dest: String,
) -> Res<MoveOutcome> {
    // A watched folder cannot be renamed on Windows.
    state.watch(None);
    let _write = state.write();
    let moved = project::relocate(Path::new(&root), Path::new(&dest)).map_err(fail)?;
    let overview = project::open(Path::new(&moved.root)).map_err(fail)?;
    let recent = recent_file(&app)?;
    let _ = recent::remove(&recent, &root);
    let _ = recent::touch(&recent, &overview);
    Ok(MoveOutcome {
        overview,
        left_behind: moved.left_behind,
    })
}
