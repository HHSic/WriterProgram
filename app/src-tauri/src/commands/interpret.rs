//! 작품 해석 (해석 모드): a chapter's reading, written by an assistant through
//! writer-mcp, with each sentence placed in the text as it is now.

use std::path::Path;

use writer_core::reading::{self, ReadingView};

use crate::error::{Res, fail};

#[tauri::command]
pub async fn interpret_load(root: String, doc_id: String) -> Res<Option<ReadingView>> {
    reading::load(Path::new(&root), &doc_id).map_err(fail)
}
