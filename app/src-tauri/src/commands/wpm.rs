//! Native project files; working copies remain available after a failed save.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use writer_core::{project, recent, store, wpm};

use crate::error::{Res, fail};
use crate::paths::recent_file;
use crate::state::AppState;

#[derive(Clone, Serialize, Deserialize)]
struct Link {
    file: String,
    revision: String,
}

fn links_file(app: &AppHandle) -> Res<PathBuf> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("wpm-links.json"))
}

fn read_links(app: &AppHandle) -> Res<BTreeMap<String, Link>> {
    let file = links_file(app)?;
    if !file.exists() {
        return Ok(BTreeMap::new());
    }
    let text = store::read_text(&file).map_err(fail)?;
    serde_json::from_str(&text).map_err(|_| "작품 파일의 저장 위치 정보를 읽지 못함".into())
}

fn write_links(app: &AppHandle, links: &BTreeMap<String, Link>) -> Res<()> {
    let bytes = serde_json::to_vec_pretty(links).expect("links serialize");
    store::atomic_write(&links_file(app)?, &bytes).map_err(fail)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileOverview {
    #[serde(flatten)]
    overview: project::Overview,
    project_file: Option<String>,
}

#[tauri::command]
pub async fn project_file_startup(state: State<'_, AppState>) -> Res<Option<String>> {
    Ok(state.take_startup_file())
}

/// A durable, independently openable package when the normal destination fails.
#[tauri::command]
pub async fn project_file_rescue(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
) -> Res<String> {
    let _write = state.write();
    let root = PathBuf::from(root);
    let project = project::load(&root).map_err(fail)?;
    // Generated directory names only; a damaged project must not escape app data.
    if project.id.is_empty()
        || !project
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err("작품의 식별자가 올바르지 않음".into());
    }
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("rescue")
        .join(&project.id);
    let file = dir.join(format!("작품-{}.wpm", store::new_id()));
    writer_core::journal::flush(&root);
    wpm::save(&root, &file).map_err(fail)?;
    Ok(file.to_string_lossy().into_owned())
}

pub fn with_file(app: &AppHandle, overview: project::Overview) -> Res<FileOverview> {
    let canonical = std::fs::canonicalize(&overview.root).map_err(|e| e.to_string())?;
    let key = canonical.to_string_lossy();
    let project_file = read_links(app)?
        .get(key.as_ref())
        .map(|link| link.file.clone());
    Ok(FileOverview {
        overview,
        project_file,
    })
}

/// Preserve a file's saving connection when its working folder is relocated.
pub fn move_link(app: &AppHandle, from: &str, to: &str) -> Res<()> {
    let mut links = read_links(app)?;
    if let Some(link) = links.remove(from) {
        let to = std::fs::canonicalize(to).map_err(|e| e.to_string())?;
        links.insert(to.to_string_lossy().into_owned(), link);
        write_links(app, &links)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn project_file_open(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Res<FileOverview> {
    let _write = state.write();
    let source = std::fs::canonicalize(&path).map_err(|_| "작품 파일을 찾을 수 없음")?;
    let revision = wpm::fingerprint(&source).map_err(fail)?;
    let parent = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("wpm-work");
    let root = wpm::unpack(&source, &parent).map_err(fail)?;
    let result = (|| {
        // Detect an external replacement while the file was being unpacked.
        if wpm::fingerprint(&source).map_err(fail)? != revision {
            return Err("여는 동안 작품 파일이 바뀌었습니다. 다시 열어 주세요.".into());
        }
        let overview = project::open(&root).map_err(fail)?;
        let canonical = std::fs::canonicalize(&root).map_err(|e| e.to_string())?;
        let mut links = read_links(&app)?;
        let file = source.to_string_lossy().into_owned();
        links.insert(
            canonical.to_string_lossy().into_owned(),
            Link {
                file: file.clone(),
                revision,
            },
        );
        write_links(&app, &links)?;
        let _ = recent::touch(&recent_file(&app)?, &overview);
        Ok(FileOverview {
            overview,
            project_file: Some(file),
        })
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(root);
    }
    result
}

/// None updates the linked file, with a lease against external changes.
/// A chosen destination is Save As, already confirmed by the file dialog.
#[tauri::command]
pub async fn project_file_save(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
    dest: Option<String>,
) -> Res<String> {
    let _write = state.write();
    let root = std::fs::canonicalize(&root).map_err(|_| "작품 폴더를 찾을 수 없음")?;
    let key = root.to_string_lossy().into_owned();
    let mut links = read_links(&app)?;
    let chosen = dest.is_some();
    let file = match dest {
        Some(dest) => {
            let mut path = PathBuf::from(dest);
            if path.extension().is_none() {
                path.set_extension("wpm");
            }
            path
        }
        None => PathBuf::from(
            &links
                .get(&key)
                .ok_or("작품 파일의 저장 위치를 먼저 골라 주세요")?
                .file,
        ),
    };
    if !chosen {
        let link = links.get(&key).expect("linked above");
        if wpm::fingerprint(&file).map_err(fail)? != link.revision {
            return Err("다른 프로그램에서 작품 파일이 바뀌어 덮어쓰지 않았습니다. 다른 이름으로 저장해 주세요.".into());
        }
    }
    writer_core::journal::flush(&root);
    wpm::save(&root, &file).map_err(fail)?;
    let file = std::fs::canonicalize(&file).map_err(|e| e.to_string())?;
    let revision = wpm::fingerprint(&file).map_err(fail)?;
    let name = file.to_string_lossy().into_owned();
    links.insert(
        key,
        Link {
            file: name.clone(),
            revision,
        },
    );
    write_links(&app, &links)?;
    Ok(name)
}
