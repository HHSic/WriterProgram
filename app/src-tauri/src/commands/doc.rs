//! Documents: adding, moving, loading and saving them, their records
//! (snapshots) and the trash.

use std::path::Path;

use chrono::Duration;
use serde::Serialize;
use tauri::State;
use writer_core::count::{Counts, count_blocks};
use writer_core::doc::{self, DocFile, DocMeta, MetaPatch, SaveGuard, SaveOutcome};
use writer_core::markup::Body;
use writer_core::project::{self, NewDoc};
use writer_core::snapshot::{self, SnapshotInfo};
use writer_core::trash::{self, TrashItem};

use crate::error::{Res, fail};
use crate::state::AppState;

/// How often an automatic record is kept while writing.
const AUTO_RECORD_EVERY_MINUTES: i64 = 10;

/// A document with its text for the editor.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocData {
    meta: DocMeta,
    body: Body,
    counts: Counts,
    /// Fingerprint of the text, sent back with saves (see `SaveGuard`).
    rev: String,
}

impl From<DocFile> for DocData {
    fn from(file: DocFile) -> Self {
        let counts = count_blocks(&file.body);
        let rev = file.rev();
        DocData {
            meta: file.meta,
            body: Body::new(file.body),
            counts,
            rev,
        }
    }
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

/// Saves the editor's text. `base` is the fingerprint of the text the editor
/// started from: when the text on disk changed since (another device), the
/// save is refused unless `force` (see `doc::save_body`).
#[tauri::command]
pub async fn doc_save(
    state: State<'_, AppState>,
    root: String,
    doc_id: String,
    body: Body,
    base: Option<String>,
    force: Option<bool>,
) -> Res<SaveOutcome> {
    let _write = state.write();
    doc::save_body(
        Path::new(&root),
        &doc_id,
        body.content,
        Duration::minutes(AUTO_RECORD_EVERY_MINUTES),
        SaveGuard {
            base: base.as_deref(),
            force: force.unwrap_or(false),
        },
    )
    .map_err(fail)
}

/// Keeps the editor's text as a record, before another device's text is
/// loaded in its place.
#[tauri::command]
pub async fn doc_keep(
    state: State<'_, AppState>,
    root: String,
    doc_id: String,
    body: Body,
    kind: String,
) -> Res<Option<SnapshotInfo>> {
    let _write = state.write();
    doc::keep_record(Path::new(&root), &doc_id, body.content, &kind).map_err(fail)
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
