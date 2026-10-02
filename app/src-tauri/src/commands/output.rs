//! Text and files out (내보내기), manuscript formats, and files in (가져오기).

use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};
use writer_core::export::{self, DocOptions, ExportItem, FileKind, TextOptions};
use writer_core::format::{self, Catalog, ManuscriptFormat, UserPreset};
use writer_core::import::{self, CommitSpec, Committed, ImportOptions, Preview};
use writer_core::project;

use crate::error::{Res, fail};
use crate::paths::presets_file;
use crate::state::AppState;

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

/// Built-in and user manuscript formats, fonts and paper sizes.
#[tauri::command]
pub async fn format_catalog(app: AppHandle) -> Res<Catalog> {
    Ok(format::catalog(&presets_file(&app)?))
}

/// "내 서식으로 저장".
#[tauri::command]
pub async fn format_save_preset(
    app: AppHandle,
    name: String,
    format: ManuscriptFormat,
) -> Res<Vec<UserPreset>> {
    format::save_user_preset(&presets_file(&app)?, &name, &format).map_err(fail)
}

#[tauri::command]
pub async fn format_delete_preset(app: AppHandle, name: String) -> Res<Vec<UserPreset>> {
    format::delete_user_preset(&presets_file(&app)?, &name).map_err(fail)
}

/// Estimated pages of the whole manuscript in a format being tried.
#[tauri::command]
pub async fn format_estimate(root: String, format: ManuscriptFormat) -> Res<Option<u32>> {
    project::estimate_pages(Path::new(&root), &format).map_err(fail)
}

/// Word (docx) or 한글 (HWPX) export with a manuscript format.
#[tauri::command]
pub async fn export_file(
    root: String,
    items: Vec<ExportItem>,
    opts: DocOptions,
    format: ManuscriptFormat,
    kind: FileKind,
    dest: String,
    per_doc: bool,
) -> Res<Vec<String>> {
    export::export_file(
        Path::new(&root),
        &items,
        &opts,
        &format,
        kind,
        Path::new(&dest),
        per_doc,
    )
    .map(|files| {
        files
            .into_iter()
            .map(|f| f.to_string_lossy().into_owned())
            .collect()
    })
    .map_err(fail)
}

/// 가져오기 미리보기: what the files would become, without making anything.
#[tauri::command]
pub async fn import_preview(paths: Vec<String>, opts: ImportOptions) -> Res<Preview> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    Ok(import::preview(&paths, &opts))
}

/// Makes the chapters picked in the preview.
#[tauri::command]
pub async fn import_commit(
    state: State<'_, AppState>,
    root: String,
    paths: Vec<String>,
    opts: ImportOptions,
    spec: CommitSpec,
) -> Res<Committed> {
    let _write = state.write();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    import::commit(Path::new(&root), &paths, &opts, &spec).map_err(fail)
}
