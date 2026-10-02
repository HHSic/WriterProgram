//! Creation proof certificate (창작 과정 증명서): the summary sentence for
//! the dialog, and making the page and the proof file in a folder the
//! writer picks (`writer_core::proof`).

use std::path::Path;

use writer_core::proof::{self, Options, Written};

use crate::error::{Res, fail};

/// The summary sentence the certificate would open with.
#[tauri::command]
pub async fn proof_preview(root: String, options: Options) -> Res<String> {
    tauri::async_runtime::spawn_blocking(move || {
        proof::preview(Path::new(&root), &options).map_err(fail)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Makes the certificate in a new folder inside `folder`. Only reads the
/// project; the files go outside it.
#[tauri::command]
pub async fn proof_make(root: String, options: Options, folder: String) -> Res<Written> {
    tauri::async_runtime::spawn_blocking(move || {
        proof::write(Path::new(&root), &options, Path::new(&folder)).map_err(fail)
    })
    .await
    .map_err(|e| e.to_string())?
}
