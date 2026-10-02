//! 교정본 주고받기: sending chapters to an editor, reading the corrected
//! file, and accepting or rejecting the corrections.

use std::path::Path;

use tauri::State;
use writer_core::corrections::{self, Applied, Decisions, Exchange, ExchangeInfo, Review};
use writer_core::export::{DocOptions, ExportItem, FileKind};
use writer_core::format::ManuscriptFormat;

use crate::error::{Res, fail};
use crate::state::AppState;

/// "편집자에게 보내기": exports like `export_file` and keeps what was sent.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn exchange_send(
    state: State<'_, AppState>,
    root: String,
    items: Vec<ExportItem>,
    opts: DocOptions,
    format: ManuscriptFormat,
    kind: FileKind,
    dest: String,
    per_doc: bool,
) -> Res<Exchange> {
    let _write = state.write();
    corrections::send(
        Path::new(&root),
        &items,
        &opts,
        &format,
        kind,
        Path::new(&dest),
        per_doc,
    )
    .map_err(fail)
}

/// Chapters sent to editors, newest first.
#[tauri::command]
pub async fn exchange_list(root: String) -> Res<Vec<ExchangeInfo>> {
    corrections::list(Path::new(&root)).map_err(fail)
}

/// Reads a corrected file and compares it with what was sent.
#[tauri::command]
pub async fn exchange_read(
    state: State<'_, AppState>,
    root: String,
    exchange_id: String,
    path: String,
) -> Res<Review> {
    let _write = state.write();
    corrections::read_corrected(Path::new(&root), &exchange_id, Path::new(&path)).map_err(fail)
}

/// The last corrected file read for an exchange, with the decisions so far.
#[tauri::command]
pub async fn exchange_review(root: String, exchange_id: String) -> Res<Option<Review>> {
    corrections::load_review(Path::new(&root), &exchange_id).map_err(fail)
}

/// Accepts or rejects corrections.
#[tauri::command]
pub async fn exchange_apply(
    state: State<'_, AppState>,
    root: String,
    exchange_id: String,
    decisions: Decisions,
) -> Res<Applied> {
    let _write = state.write();
    corrections::apply(Path::new(&root), &exchange_id, &decisions).map_err(fail)
}
