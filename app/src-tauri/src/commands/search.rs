//! 작품 전체 찾기 and 모두 바꾸기.

use std::path::Path;

use tauri::State;
use writer_core::search::{self, ReplaceOutcome, SearchQuery, SearchResult};

use crate::error::{Res, fail};
use crate::state::AppState;

/// 작품 전체 찾기.
#[tauri::command]
pub async fn search(root: String, query: SearchQuery) -> Res<SearchResult> {
    search::search(Path::new(&root), &query).map_err(fail)
}

/// 모두 바꾸기. Each changed document first gets a "바꾸기 전" record.
#[tauri::command]
pub async fn replace_all(
    state: State<'_, AppState>,
    root: String,
    query: SearchQuery,
    replacement: String,
) -> Res<ReplaceOutcome> {
    let _write = state.write();
    search::replace_all(Path::new(&root), &query, &replacement).map_err(fail)
}
