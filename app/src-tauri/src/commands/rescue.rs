//! Rescue copies (비상 보관): writing that kept failing to save into the
//! project folder, kept in the app's data folder (`writer_core::rescue`).

use chrono::{DateTime, Local};
use tauri::AppHandle;
use writer_core::markup::{Body, write_body};
use writer_core::rescue::{self, RescueFile};

use crate::error::{Res, fail};
use crate::paths::rescue_dir;

/// Writes a rescue copy of `item` and answers its path. A chapter's text
/// comes as `body`, anything else as `text`; `since` (milliseconds since
/// 1970) is when the saves started failing, which names the file.
#[tauri::command]
pub async fn rescue_save(
    app: AppHandle,
    project_id: String,
    item: String,
    since: i64,
    body: Option<Body>,
    text: Option<String>,
) -> Res<String> {
    let base = rescue_dir(&app)?;
    let at = DateTime::from_timestamp_millis(since)
        .map(|t| t.with_timezone(&Local))
        .unwrap_or_else(Local::now);
    let text = match body {
        Some(body) => write_body(&body.content),
        None => text.unwrap_or_default(),
    };
    let path = rescue::write(&base, &project_id, &item, &rescue::stamp(at), &text).map_err(fail)?;
    Ok(path.to_string_lossy().into_owned())
}

/// The project's rescue copies not dealt with yet, newest first.
#[tauri::command]
pub async fn rescue_list(app: AppHandle, project_id: String) -> Res<Vec<RescueFile>> {
    rescue::list(&rescue_dir(&app)?, &project_id).map_err(fail)
}

/// The text of a chapter's rescue copy, for comparing.
#[tauri::command]
pub async fn rescue_load(app: AppHandle, path: String) -> Res<Body> {
    let blocks = rescue::load_body(&rescue_dir(&app)?, &path).map_err(fail)?;
    Ok(Body::new(blocks))
}

/// Moves a rescue copy the writer has dealt with out of the list (kept in `old/`).
#[tauri::command]
pub async fn rescue_set_aside(app: AppHandle, path: String) -> Res<()> {
    rescue::set_aside(&rescue_dir(&app)?, &path).map_err(fail)
}

/// The rescue folder of a project, for 위치 열기.
#[tauri::command]
pub async fn rescue_folder(app: AppHandle, project_id: String) -> Res<String> {
    let dir = rescue::folder(&rescue_dir(&app)?, &project_id).map_err(fail)?;
    Ok(dir.to_string_lossy().into_owned())
}
