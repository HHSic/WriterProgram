//! A project whose `project.json` is damaged: what can bring it back, and
//! doing it (`writer_core::project::recover`, docs/safety-design.md S5).

use std::path::Path;

use tauri::{AppHandle, State};
use writer_core::project::{self, Overview, Recovery, Way};
use writer_core::recent;

use crate::error::{Res, fail};
use crate::paths::recent_file;
use crate::state::AppState;

/// What can bring the project back; none when its project.json is fine.
/// The screen asks after opening fails.
#[tauri::command]
pub async fn project_recovery(path: String) -> Res<Option<Recovery>> {
    project::recovery(Path::new(&path)).map_err(fail)
}

/// Brings project.json back the chosen way and opens the project.
#[tauri::command]
pub async fn project_recover(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    way: Way,
) -> Res<Overview> {
    let _write = state.write();
    let root = Path::new(&path);
    project::recover(root, way).map_err(fail)?;
    let overview = project::open(root).map_err(fail)?;
    let _ = recent::touch(&recent_file(&app)?, &overview);
    Ok(overview)
}
