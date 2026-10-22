//! 작품 해석 (해석 모드): a chapter's reading, written by an assistant through
//! writer-mcp, with each sentence placed in the text as it is now; and the
//! writer's own changes to it.

use std::path::Path;

use writer_core::reading::{self, Reading, ReadingView};

use crate::error::{Res, fail};

#[tauri::command]
pub async fn interpret_load(root: String, doc_id: String) -> Res<Option<ReadingView>> {
    reading::load(Path::new(&root), &doc_id).map_err(fail)
}

/// Keeps the writer's changes and gives the reading back placed in the text.
#[tauri::command]
pub async fn interpret_save(root: String, reading: Reading) -> Res<Option<ReadingView>> {
    let root = Path::new(&root);
    let doc_id = reading.doc.clone();
    reading::save_edit(root, reading).map_err(fail)?;
    reading::load(root, &doc_id).map_err(fail)
}
