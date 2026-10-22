//! Projects: the recent list, making, opening, changing and moving a
//! project, its parts, and showing files in the file manager.

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;
use writer_core::project::{self, NewProject, Overview, ProjectInfo, ProjectPatch, Sizes};
use writer_core::recent::{self, RecentItem};

use super::wpm::{FileOverview, with_file};
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
) -> Res<FileOverview> {
    let _write = state.write();
    let overview = project::open(Path::new(&path)).map_err(fail)?;
    let _ = recent::touch(&recent_file(&app)?, &overview);
    with_file(&app, overview)
}

#[tauri::command]
pub async fn project_overview(app: AppHandle, root: String) -> Res<FileOverview> {
    with_file(&app, project::overview(Path::new(&root)).map_err(fail)?)
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

/// 이 작품 크기: the writing, records, trash and journal, whether to suggest
/// tidying records, and the free room on the project's disk.
#[tauri::command]
pub async fn project_size(root: String) -> Res<Sizes> {
    project::sizes(Path::new(&root)).map_err(fail)
}

/// writer-mcp next to the app, where the installer puts it, for connecting an
/// AI assistant to a project (docs/mcp.md). None when it is not there.
#[tauri::command]
pub async fn mcp_server() -> Res<Option<String>> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let name = if cfg!(windows) {
        "writer-mcp.exe"
    } else {
        "writer-mcp"
    };
    Ok(exe
        .parent()
        .map(|dir| dir.join(name))
        .filter(|path| path.is_file())
        .map(|path| path.display().to_string()))
}

/// The `writer` command next to the app (docs/cli.md) and whether its folder
/// is on the user's PATH, so a terminal finds it by name.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliCommand {
    path: String,
    on: bool,
    /// PATH can be changed from here (Windows); elsewhere the shell's own
    /// settings do it.
    can_change: bool,
}

fn cli_command_now() -> Res<Option<CliCommand>> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let Some(dir) = exe.parent() else {
        return Ok(None);
    };
    let path = dir.join(if cfg!(windows) {
        "writer.exe"
    } else {
        "writer"
    });
    if !path.is_file() {
        return Ok(None);
    }
    let dir = writer_mcp::path::shown(dir);
    Ok(Some(CliCommand {
        on: writer_mcp::path::is_on(&dir)?,
        path: writer_mcp::path::shown(&path),
        can_change: cfg!(windows),
    }))
}

/// None when `writer` is not next to the app, as in a build without it.
#[tauri::command]
pub async fn cli_command() -> Res<Option<CliCommand>> {
    cli_command_now()
}

/// Puts the app's folder on the user's PATH or takes it off (Windows).
#[tauri::command]
pub async fn cli_path_set(on: bool) -> Res<Option<CliCommand>> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().ok_or("앱 폴더를 알 수 없음")?;
    writer_mcp::path::set(&writer_mcp::path::shown(dir), on)?;
    cli_command_now()
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
    overview: FileOverview,
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
    let old = std::fs::canonicalize(&root).map_err(|e| e.to_string())?;
    let moved = project::relocate(Path::new(&root), Path::new(&dest)).map_err(fail)?;
    let overview = project::open(Path::new(&moved.root)).map_err(fail)?;
    super::wpm::move_link(&app, &old.to_string_lossy(), &overview.root)?;
    let recent = recent_file(&app)?;
    let _ = recent::remove(&recent, &root);
    let _ = recent::touch(&recent, &overview);
    Ok(MoveOutcome {
        overview: with_file(&app, overview)?,
        left_behind: moved.left_behind,
    })
}
