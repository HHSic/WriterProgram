//! Notes (메모).

use std::path::Path;

use tauri::State;
use writer_core::notes::{self, NewNote, Note};
use writer_core::trash::{self, TrashItem};

use crate::error::{Res, fail};
use crate::state::AppState;

#[tauri::command]
pub async fn note_list(root: String) -> Res<Vec<Note>> {
    notes::list(Path::new(&root)).map_err(fail)
}

#[tauri::command]
pub async fn note_create(state: State<'_, AppState>, root: String, spec: NewNote) -> Res<Note> {
    let _write = state.write();
    notes::create(Path::new(&root), &spec).map_err(fail)
}

#[tauri::command]
pub async fn note_save(state: State<'_, AppState>, root: String, note: Note) -> Res<Note> {
    let _write = state.write();
    notes::save(Path::new(&root), &note).map_err(fail)
}

#[tauri::command]
pub async fn note_trash(
    state: State<'_, AppState>,
    root: String,
    note_id: String,
) -> Res<TrashItem> {
    let _write = state.write();
    trash::trash_note(Path::new(&root), &note_id).map_err(fail)
}
