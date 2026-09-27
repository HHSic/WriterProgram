//! Commands the screen calls. Each one is a thin wrapper over `writer_core`;
//! errors come back as a short reason in screen words.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use chrono::Duration;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;
use writer_core::count::{Counts, count_blocks};
use writer_core::doc::{self, DocFile, DocMeta, MetaPatch, SaveOutcome};
use writer_core::export::{self, ExportItem, TextOptions};
use writer_core::markup::Body;
use writer_core::project::{self, NewDoc, NewProject, Overview, ProjectInfo, ProjectPatch};
use writer_core::recent::{self, RecentItem};
use writer_core::snapshot::{self, SnapshotInfo};
use writer_core::trash::{self, TrashItem};

/// How often an automatic record is kept while writing.
const AUTO_RECORD_EVERY_MINUTES: i64 = 10;

/// Serializes writes so two saves never read-modify-write the same file at once.
#[derive(Default)]
pub struct AppState {
    lock: Mutex<()>,
}

impl AppState {
    fn write(&self) -> MutexGuard<'_, ()> {
        self.lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

type Res<T> = Result<T, String>;

fn fail(e: writer_core::Error) -> String {
    e.user_message()
}

fn recent_file(app: &AppHandle) -> Res<PathBuf> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("recent.json"))
        .map_err(|e| e.to_string())
}

/// A document with its text for the editor.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocData {
    meta: DocMeta,
    body: Body,
    counts: Counts,
}

impl From<DocFile> for DocData {
    fn from(file: DocFile) -> Self {
        let counts = count_blocks(&file.body);
        DocData {
            meta: file.meta,
            body: Body::new(file.body),
            counts,
        }
    }
}

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

#[tauri::command]
pub async fn doc_add(state: State<'_, AppState>, root: String, spec: NewDoc) -> Res<String> {
    let _write = state.write();
    project::add_doc(Path::new(&root), &spec).map_err(fail)
}

#[tauri::command]
pub async fn doc_move(
    state: State<'_, AppState>,
    root: String,
    doc_id: String,
    part_id: Option<String>,
    index: usize,
) -> Res<()> {
    let _write = state.write();
    project::move_doc(Path::new(&root), &doc_id, part_id.as_deref(), index).map_err(fail)
}

#[tauri::command]
pub async fn doc_trash(state: State<'_, AppState>, root: String, doc_id: String) -> Res<TrashItem> {
    let _write = state.write();
    trash::trash_doc(Path::new(&root), &doc_id).map_err(fail)
}

#[tauri::command]
pub async fn doc_load(root: String, doc_id: String) -> Res<DocData> {
    doc::load(Path::new(&root), &doc_id)
        .map(DocData::from)
        .map_err(fail)
}

#[tauri::command]
pub async fn doc_save(
    state: State<'_, AppState>,
    root: String,
    doc_id: String,
    body: Body,
) -> Res<SaveOutcome> {
    let _write = state.write();
    doc::save_body(
        Path::new(&root),
        &doc_id,
        body.content,
        Duration::minutes(AUTO_RECORD_EVERY_MINUTES),
    )
    .map_err(fail)
}

#[tauri::command]
pub async fn doc_update_meta(
    state: State<'_, AppState>,
    root: String,
    doc_id: String,
    patch: MetaPatch,
) -> Res<DocMeta> {
    let _write = state.write();
    doc::update_meta(Path::new(&root), &doc_id, &patch).map_err(fail)
}

#[tauri::command]
pub async fn snapshot_list(root: String, doc_id: String) -> Res<Vec<SnapshotInfo>> {
    snapshot::list(Path::new(&root), &doc_id).map_err(fail)
}

/// "지금 원고 보관": keeps a record by hand.
#[tauri::command]
pub async fn snapshot_create(
    state: State<'_, AppState>,
    root: String,
    doc_id: String,
    name: String,
) -> Res<SnapshotInfo> {
    let _write = state.write();
    let root = Path::new(&root);
    let current = doc::load(root, &doc_id).map_err(fail)?;
    snapshot::create(root, &current, "manual", &name).map_err(fail)
}

#[tauri::command]
pub async fn snapshot_load(root: String, doc_id: String, snapshot_id: String) -> Res<DocData> {
    snapshot::load(Path::new(&root), &doc_id, &snapshot_id)
        .map(DocData::from)
        .map_err(fail)
}

/// "이 때로 되돌리기".
#[tauri::command]
pub async fn snapshot_restore(
    state: State<'_, AppState>,
    root: String,
    doc_id: String,
    snapshot_id: String,
) -> Res<SnapshotInfo> {
    let _write = state.write();
    snapshot::restore(Path::new(&root), &doc_id, &snapshot_id).map_err(fail)
}

#[tauri::command]
pub async fn trash_list(root: String) -> Res<Vec<TrashItem>> {
    trash::list(Path::new(&root)).map_err(fail)
}

#[tauri::command]
pub async fn trash_restore(state: State<'_, AppState>, root: String, trash_id: String) -> Res<()> {
    let _write = state.write();
    trash::restore(Path::new(&root), &trash_id)
        .map(|_| ())
        .map_err(fail)
}

#[tauri::command]
pub async fn trash_delete(state: State<'_, AppState>, root: String, trash_id: String) -> Res<()> {
    let _write = state.write();
    trash::delete(Path::new(&root), &trash_id).map_err(fail)
}

/// Text for the clipboard.
#[tauri::command]
pub async fn export_text(root: String, items: Vec<ExportItem>, opts: TextOptions) -> Res<String> {
    export::items_text(Path::new(&root), &items, &opts).map_err(fail)
}

#[tauri::command]
pub async fn export_txt(
    root: String,
    items: Vec<ExportItem>,
    opts: TextOptions,
    dest: String,
    per_doc: bool,
) -> Res<Vec<String>> {
    export::export_txt(Path::new(&root), &items, &opts, Path::new(&dest), per_doc)
        .map(|files| {
            files
                .into_iter()
                .map(|f| f.to_string_lossy().into_owned())
                .collect()
        })
        .map_err(fail)
}

/// Shows a file or folder in the system file manager.
#[tauri::command]
pub async fn reveal(app: AppHandle, path: String) -> Res<()> {
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| e.to_string())
}
