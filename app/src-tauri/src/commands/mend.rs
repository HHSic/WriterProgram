//! 고쳐 열기: chapters whose file is there but cannot be read
//! (`writer_core::mend`, docs/safety-design.md S4).

use std::path::Path;

use tauri::State;
use writer_core::mend::{self, MendPreview, Mended};

use crate::error::{Res, fail};
use crate::state::AppState;

/// How the chapter would read once mended, before the writer agrees.
#[tauri::command]
pub async fn doc_mend_preview(root: String, doc_id: String) -> Res<MendPreview> {
    mend::preview(Path::new(&root), &doc_id).map_err(fail)
}

/// Keeps the original next to it and writes the chapter again as UTF-8.
#[tauri::command]
pub async fn doc_mend(state: State<'_, AppState>, root: String, doc_id: String) -> Res<Mended> {
    let _write = state.write();
    mend::mend(Path::new(&root), &doc_id).map_err(fail)
}
